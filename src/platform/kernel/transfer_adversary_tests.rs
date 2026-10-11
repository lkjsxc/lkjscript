//! Transfer proofs must account for inactive cases, phantom actuals and exact scopes.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{
    CaseId, DeclarationId, ExpressionId, ModuleId, TypeParameterId,
};
use std::cell::Cell;
use std::collections::BTreeMap;

const SEED: &[u8] = b"transfer-proof-adversarial-scopes-and-recursion";

fn declaration(index: u64) -> DeclarationId {
    DeclarationId::migrate(SEED, index)
}

fn parameter(index: u64) -> TypeParameterId {
    TypeParameterId::migrate(SEED, index)
}

fn package(index: u64) -> PackageId {
    PackageId::migrate(SEED, index)
}

struct Read {
    owners: BTreeMap<OwnerKey, OwnerRecord>,
    imports: BTreeMap<(PackageId, OwnerKey), PackageInterfaceRecord>,
    interner: TypeObjectInterner,
    steps: Cell<usize>,
    reads: Cell<usize>,
    maximum_steps: usize,
}

impl Default for Read {
    fn default() -> Self {
        Self {
            owners: BTreeMap::new(),
            imports: BTreeMap::new(),
            interner: TypeObjectInterner::default(),
            steps: Cell::new(0),
            reads: Cell::new(0),
            maximum_steps: contract::MAXIMUM_VALIDATION_WORK,
        }
    }
}

impl ExpressionRead for Read {
    fn package_id(&self) -> PackageId {
        package(0)
    }

    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.owners.get(&owner).cloned())
    }

    fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.interner.get(ty).cloned())
    }

    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.imports.get(&(package, owner)).cloned())
    }

    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        Ok(self.imports.keys().any(|(p, _)| *p == package))
    }

    fn validation_work(&self) -> Result<(), Diagnostic> {
        if self.steps.get() >= self.maximum_steps {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "transfer_adversary_work",
                "isolated transfer proof exhausted its admitted work",
            ));
        }
        self.steps.set(self.steps.get() + 1);
        Ok(())
    }
}

impl Read {
    fn ty(&mut self, form: TypeForm) -> TypeObjectDigest {
        self.interner.intern(form).unwrap()
    }

    fn local(&mut self, record: OwnerRecord) {
        self.owners.insert(record.owner(), record);
    }

    fn export(&mut self, index: u64) {
        let records: Vec<_> = self.owners.values().cloned().collect();
        for record in records {
            if let Some(projected) = PackageInterfaceRecord::project_public(&record).unwrap() {
                self.imports
                    .insert((package(index), record.owner()), projected);
            }
        }
    }

    fn function(&mut self, index: u64, parameters: &[(u64, TypeParameterConstraints)]) {
        let result = self.ty(TypeForm::I64);
        self.local(OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(declaration(index)),
                OwnerKind::PureFunction,
            ),
            module: ModuleId::migrate(SEED, 0),
            name: Name::new(format!("function-{index}")).unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                result_borrow: None,
                implementation_parameters: Vec::new(),
                requirement_parameters: Vec::new(),
                effect_parameters: Vec::new(),
                type_parameters: parameters.iter().map(|(id, _)| parameter(*id)).collect(),
                parameters: Vec::new(),
                result,
                effect: FunctionEffect::Pure,
                body: ExpressionId::migrate(SEED, index),
            }),
        }));
        for (id, constraint) in parameters {
            self.type_parameter(index, *id, *constraint);
        }
    }

    fn type_parameter(&mut self, owner: u64, id: u64, constraints: TypeParameterConstraints) {
        self.local(OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(
                OwnerKey::TypeParameter(parameter(id)),
                OwnerKind::TypeParameter,
            ),
            declaration: declaration(owner),
            name: Name::new(format!("T{id}")).unwrap(),
            constraints,
        }));
    }

    fn variant(&mut self, index: u64, parameters: &[u64], payloads: &[Option<TypeObjectDigest>]) {
        let mut cases = Vec::new();
        for (position, payload) in payloads.iter().enumerate() {
            let id = CaseId::migrate(SEED, index * 16 + position as u64);
            self.local(OwnerRecord::Case(CaseRecord {
                header: OwnerHeader::new(OwnerKey::Case(id), OwnerKind::Case),
                declaration: declaration(index),
                name: Name::new(format!("case-{position}")).unwrap(),
                payload: *payload,
            }));
            cases.push(id);
        }
        self.local(OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(declaration(index)),
                OwnerKind::Variant,
            ),
            module: ModuleId::migrate(SEED, 0),
            name: Name::new(format!("variant-{index}")).unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Variant {
                type_parameters: parameters.iter().map(|id| parameter(*id)).collect(),
                cases,
            },
        }));
        for id in parameters {
            self.type_parameter(index, *id, TypeParameterConstraints::None);
        }
    }

    fn applied(
        &mut self,
        package_index: u64,
        index: u64,
        arguments: Vec<TypeObjectDigest>,
    ) -> TypeObjectDigest {
        let declaration = DeclarationReference {
            package: package(package_index),
            declaration: declaration(index),
        };
        self.ty(if arguments.is_empty() {
            TypeForm::Named { declaration }
        } else {
            TypeForm::Applied {
                declaration,
                arguments,
            }
        })
    }
}

#[test]
fn transfer_recursive_nominals_reject_inactive_authority_in_either_traversal_order() {
    for unsafe_case_first in [false, true] {
        let mut read = Read::default();
        let a = read.applied(0, 0, Vec::new());
        let b = read.applied(0, 1, Vec::new());
        let scalar = read.ty(TypeForm::I64);
        let callback = read.ty(TypeForm::Function {
            parameters: Vec::new(),
            result: scalar,
        });
        read.variant(1, &[], &[None, Some(a)]);
        let payloads = if unsafe_case_first {
            [Some(callback), Some(b), None]
        } else {
            [None, Some(b), Some(callback)]
        };
        read.variant(0, &[], &payloads);
        // A Stop value contains no callback, but its complete type closure does.
        for root in [a, b] {
            assert!(
                transfer::admit(&read, root, None).is_err(),
                "order={unsafe_case_first}, root={root}"
            );
        }
    }
    let mut read = Read::default();
    let a = read.applied(0, 0, Vec::new());
    let b = read.applied(0, 1, Vec::new());
    read.variant(0, &[], &[None, Some(b)]);
    read.variant(1, &[], &[None, Some(a)]);
    for root in [a, b] {
        assert!(!transfer::admit(&read, root, None).unwrap());
    }
}

#[test]
fn transfer_recursive_generic_nominals_keep_actual_proofs_separate_from_formals() {
    let mut read = Read::default();
    let t = read.ty(TypeForm::TypeParameter {
        parameter: parameter(0),
    });
    let u = read.ty(TypeForm::TypeParameter {
        parameter: parameter(1),
    });
    let b_of_t = read.applied(0, 1, vec![t]);
    let a_of_u = read.applied(0, 0, vec![u]);
    read.variant(0, &[0], &[None, Some(b_of_t)]);
    read.variant(1, &[1], &[None, Some(a_of_u)]);
    let scalar = read.ty(TypeForm::I64);
    let secret = read.ty(TypeForm::Secret);
    let ordinary = read.applied(0, 0, vec![scalar]);
    let poisoned = read.applied(0, 0, vec![secret]);
    assert!(!transfer::admit(&read, ordinary, None).unwrap());
    assert!(transfer::admit(&read, poisoned, None).is_err());
    read.function(2, &[(2, TypeParameterConstraints::Transferable)]);
    let caller = read.ty(TypeForm::TypeParameter {
        parameter: parameter(2),
    });
    let symbolic = read.applied(0, 0, vec![caller]);
    assert!(!transfer::admit(&read, symbolic, Some(declaration(2))).unwrap());
    assert!(transfer::admit(&read, symbolic, None).is_err());
}

#[test]
fn transfer_phantom_actuals_cannot_hide_authority_or_owned_memory() {
    let mut read = Read::default();
    read.variant(0, &[0], &[None]);
    let scalar = read.ty(TypeForm::I64);
    let callback = read.ty(TypeForm::Function {
        parameters: Vec::new(),
        result: scalar,
    });
    let secret = read.ty(TypeForm::Secret);
    let owner = read.ty(TypeForm::ByteBuffer);
    let hidden = read.ty(TypeForm::Option { item: callback });
    for actual in [callback, secret, owner, hidden] {
        let phantom = read.applied(0, 0, vec![actual]);
        assert!(
            transfer::admit(&read, phantom, None).is_err(),
            "actual={actual}"
        );
        let sequence = read.ty(TypeForm::OwnedSequence { item: phantom });
        assert!(
            owned_product::validate(&read, sequence, None).is_err(),
            "empty sequence must check phantom actual={actual}"
        );
        assert!(transfer::admit(&read, sequence, None).is_err());
        assert!(share::admit(&read, sequence, None).is_err());
    }
    let ordinary = read.applied(0, 0, vec![scalar]);
    assert!(!transfer::admit(&read, ordinary, None).unwrap());
    let sequence = read.ty(TypeForm::OwnedSequence { item: ordinary });
    owned_product::validate(&read, sequence, None).unwrap();
    assert!(transfer::admit(&read, sequence, None).unwrap());
    assert!(share::admit(&read, sequence, None).unwrap());
}

#[test]
fn transfer_symbolic_proof_requires_exact_function_owner_and_signature_membership() {
    for (constraint, expected_owned) in [
        (TypeParameterConstraints::None, None),
        (TypeParameterConstraints::CaptureSafe, None),
        (TypeParameterConstraints::Owned, None),
        (TypeParameterConstraints::Transferable, Some(false)),
        (
            TypeParameterConstraints::CaptureSafeTransferable,
            Some(false),
        ),
        (TypeParameterConstraints::OwnedTransferable, Some(true)),
    ] {
        let mut read = Read::default();
        read.function(0, &[(0, constraint)]);
        read.function(1, &[]);
        let ty = read.ty(TypeForm::TypeParameter {
            parameter: parameter(0),
        });
        let proof = transfer::admit(&read, ty, Some(declaration(0)));
        match expected_owned {
            Some(owned) => assert_eq!(proof.unwrap(), owned),
            None => assert!(proof.is_err(), "constraint={constraint:?}"),
        }
        assert!(transfer::admit(&read, ty, None).is_err());
        assert!(transfer::admit(&read, ty, Some(declaration(1))).is_err());
        // Merely changing the parameter's owner cannot insert it into another signature.
        let OwnerRecord::TypeParameter(record) = read
            .owners
            .get_mut(&OwnerKey::TypeParameter(parameter(0)))
            .unwrap()
        else {
            panic!("fixture parameter")
        };
        record.declaration = declaration(1);
        assert!(transfer::admit(&read, ty, Some(declaration(1))).is_err());
    }
}

#[test]
fn transfer_imported_nominal_assumptions_do_not_borrow_local_parameter_authority() {
    let mut read = Read::default();
    // Imported formal T0 shares its raw identity with a later local Transferable T0.
    let formal = read.ty(TypeForm::TypeParameter {
        parameter: parameter(0),
    });
    read.variant(0, &[0], &[None, Some(formal)]);
    read.variant(1, &[], &[None, Some(formal)]);
    read.export(1);
    read.owners.clear();
    read.function(2, &[(0, TypeParameterConstraints::Transferable)]);
    let scalar = read.ty(TypeForm::I64);
    let secret = read.ty(TypeForm::Secret);
    let concrete = read.applied(1, 0, vec![scalar]);
    let symbolic = read.applied(1, 0, vec![formal]);
    let poisoned = read.applied(1, 0, vec![secret]);
    assert!(!transfer::admit(&read, concrete, Some(declaration(2))).unwrap());
    assert!(!transfer::admit(&read, symbolic, Some(declaration(2))).unwrap());
    assert!(transfer::admit(&read, poisoned, Some(declaration(2))).is_err());
    // The second imported variant never declares T0; local T0 is no substitute.
    let escaped = read.applied(1, 1, Vec::new());
    assert!(transfer::admit(&read, escaped, Some(declaration(2))).is_err());
}

#[test]
fn transfer_zero_work_budget_stops_before_metadata_reads() {
    let mut read = Read::default();
    read.function(0, &[(0, TypeParameterConstraints::Transferable)]);
    let ty = read.ty(TypeForm::TypeParameter {
        parameter: parameter(0),
    });
    read.maximum_steps = 0;
    let failure = transfer::admit(&read, ty, Some(declaration(0))).unwrap_err();
    assert_eq!(failure.class, DiagnosticClass::Resource);
    assert_eq!(failure.code, "transfer_adversary_work");
    assert_eq!(read.reads.get(), 0);
}

#[test]
fn transfer_signature_membership_respects_exact_and_insufficient_work() {
    let parameters = (0..64).map(parameter).collect::<Vec<_>>();
    for target in [parameter(0), parameter(63), parameter(64)] {
        let read = Read {
            maximum_steps: parameters.len(),
            ..Read::default()
        };
        assert_eq!(
            transfer::function_parameter_listed(&read, &parameters, target).unwrap(),
            target != parameter(64)
        );
        assert_eq!(read.steps.get(), parameters.len());
        assert_eq!(read.reads.get(), 0);
        let read = Read {
            maximum_steps: parameters.len() - 1,
            ..Read::default()
        };
        let failure = transfer::function_parameter_listed(&read, &parameters, target).unwrap_err();
        assert_eq!(failure.class, DiagnosticClass::Resource);
        assert_eq!(failure.code, "transfer_adversary_work");
    }
}
