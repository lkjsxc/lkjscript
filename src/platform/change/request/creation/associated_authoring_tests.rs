//! Raw proposal lowering evidence; no accepted associated source or artifact is fabricated.
use super::super::super::{
    AuthoredChange, AuthoredLoweringInputs, ChangeBudget, SymbolDefinition, SymbolKind,
};
use super::*;
use crate::platform::change::CanonicalRead;
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{ChangeDigest, KernelSnapshot, PackageId};
use crate::platform::kernel::{
    DependencyRecord, OwnerRecord, PackageInterfaceRecord, RetirementRecord, SemanticRoot,
    TypeObject, TypeObjectDigest,
};
use crate::platform::semantic_id::RepositoryId;
use crate::platform::semantic_id::RevisionId;
use crate::platform::witness::FullWitness;

struct ControlledBase<'a> {
    base: &'a KernelSnapshot,
    control: ExecutionControl,
}

impl CanonicalBaseRead for ControlledBase<'_> {
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.control
            .check()
            .map_err(|error| Diagnostic::new(DiagnosticClass::Cancelled, error.code, error.message))
    }
    fn semantic_root(&self) -> &SemanticRoot {
        self.base.semantic_root()
    }
    fn repository_id(&self) -> RepositoryId {
        self.base.repository_id()
    }
    fn package_id(&self) -> PackageId {
        self.base.package_id()
    }
    fn exact_revision(&self) -> Option<RevisionId> {
        self.base.exact_revision()
    }
    fn owner_count(&self) -> u64 {
        self.base.owner_count()
    }
    fn dependency_count(&self) -> u64 {
        self.base.dependency_count()
    }
    fn retirement_count(&self) -> u64 {
        self.base.retirement_count()
    }
    fn read_owner(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        self.base.read_owner(owner)
    }
    fn read_type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        self.base.read_type_object(digest)
    }
    fn read_package_interface_owner(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.base.read_package_interface_owner(dependency, owner)
    }
    fn read_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        self.base.read_dependency(package)
    }
    fn read_retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
        self.base.read_retirement(owner)
    }
}

fn members() -> Vec<AuthoredAssociatedTypeMember> {
    let mut result = vec![
        AuthoredAssociatedTypeMember {
            id: AssociatedTypeMemberId::migrate(b"associated-lower", 1),
            name: Name::new("Out").unwrap(),
            bound: AssociatedTypeBound::Owned,
        },
        AuthoredAssociatedTypeMember {
            id: AssociatedTypeMemberId::migrate(b"associated-lower", 2),
            name: Name::new("Unused").unwrap(),
            bound: AssociatedTypeBound::Owned,
        },
    ];
    result.sort_by_key(|member| std::cmp::Reverse(member.id));
    result
}

fn self_type(base: &KernelSnapshot) -> AuthoredType {
    let parameter = base
        .owners
        .keys()
        .find_map(|key| match key {
            OwnerKey::TypeParameter(id) => Some(*id),
            _ => None,
        })
        .unwrap();
    AuthoredType::TypeParameter {
        parameter: AuthoredTypeParameterReference::Id { parameter },
    }
}

fn lowerer<'a>(
    base: &'a KernelSnapshot,
    witness: &'a FullWitness,
    budget: ChangeBudget,
) -> AuthoredLowerer<'a, KernelSnapshot, FullWitness> {
    AuthoredLowerer::new(
        base,
        witness,
        AuthoredLoweringInputs {
            intent_base: RevisionId::from_digest([9; 32]),
            allocation_seed: [7; 32],
            deletion_change: ChangeDigest::of(b"associated raw lowering"),
            allocated: std::collections::BTreeMap::from([
                (
                    "$contract".into(),
                    OwnerKey::Declaration(DeclarationId::migrate(b"associated-lower", 1)),
                ),
                (
                    "$implementation".into(),
                    OwnerKey::Declaration(DeclarationId::migrate(b"associated-lower", 2)),
                ),
            ]),
            definitions: std::collections::BTreeMap::from([
                (
                    "$contract".into(),
                    SymbolDefinition {
                        kind: SymbolKind::Declaration,
                        ordinal: 1,
                    },
                ),
                (
                    "$implementation".into(),
                    SymbolDefinition {
                        kind: SymbolKind::Declaration,
                        ordinal: 2,
                    },
                ),
            ]),
            allocations: vec![],
            budget,
        },
    )
    .unwrap()
}

fn value<'a>(
    lowerer: &'a AuthoredLowerer<'_, KernelSnapshot, FullWitness>,
    ordinal: u64,
) -> &'a DeclarationPayload {
    let record = &lowerer.owners
        [&OwnerKey::Declaration(DeclarationId::migrate(b"associated-lower", ordinal))]
        .record;
    let OwnerRecord::Declaration(record) = record else {
        panic!("declaration")
    };
    &record.payload
}

#[test]
fn associated_create_and_set_preserve_member_identity_order_unused_bindings_and_complete_replacement()
 {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let witness = crate::platform::witness::rebuild_full_witness(&base).unwrap();
    let mut lowerer = lowerer(&base, &witness, ChangeBudget::default());
    let module = base
        .owners
        .keys()
        .find_map(|key| match key {
            OwnerKey::Module(id) => Some(*id),
            _ => None,
        })
        .unwrap();
    let members = members();
    let bindings = vec![
        AuthoredAssociatedTypeBinding {
            member: members[0].id,
            value: AuthoredAssociatedTypeBindingValue::Owned {
                ty: AuthoredType::OwnedI64Cell {},
            },
        },
        AuthoredAssociatedTypeBinding {
            member: members[1].id,
            value: AuthoredAssociatedTypeBindingValue::Owned {
                ty: AuthoredType::ByteBuffer {},
            },
        },
    ];
    lower(
        &mut lowerer,
        &AuthoredChange::CreateOwnedContract {
            symbol: "$contract".into(),
            module: ModuleSelector::Id { module },
            name: Name::new("RawContract").unwrap(),
            visibility: DeclarationVisibility::Public,
            self_type: self_type(&base),
            type_parameters: vec![],
            methods: vec![],
            associated_types: members.clone(),
        },
    )
    .unwrap();
    let DeclarationPayload::OwnedContract(contract) = value(&lowerer, 1) else {
        panic!("contract")
    };
    assert_eq!(
        contract
            .associated_types
            .iter()
            .map(|m| (m.id, &m.name, &m.bound))
            .collect::<Vec<_>>(),
        members
            .iter()
            .map(|m| (m.id, &m.name, &m.bound))
            .collect::<Vec<_>>()
    );
    lower(
        &mut lowerer,
        &AuthoredChange::CreateOwnedImplementation {
            symbol: "$implementation".into(),
            module: ModuleSelector::Id { module },
            name: Name::new("RawImplementation").unwrap(),
            visibility: DeclarationVisibility::Public,
            contract: AuthoredDeclarationReference::Local {
                declaration: DeclarationSelector::Symbol {
                    symbol: "$contract".into(),
                },
            },
            type_parameters: vec![],
            implementation_parameters: vec![],
            self_type: AuthoredType::OwnedI64Cell {},
            type_arguments: vec![],
            methods: vec![],
            associated_types: bindings.clone(),
        },
    )
    .unwrap();
    let DeclarationPayload::OwnedImplementation(implementation) = value(&lowerer, 2) else {
        panic!("implementation")
    };
    assert_eq!(implementation.associated_types.len(), 2);
    assert!(implementation.associated_types[0].member < implementation.associated_types[1].member);
    for binding in &implementation.associated_types {
        let expected = if binding.member == members[0].id {
            TypeForm::OwnedI64Cell
        } else {
            TypeForm::ByteBuffer
        };
        let crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty } = &binding.value
        else {
            panic!("owned binding")
        };
        assert_eq!(lowerer.types.get(*ty).unwrap().form, expected);
    }
    lower(
        &mut lowerer,
        &AuthoredChange::SetOwnedContract {
            declaration: DeclarationSelector::Symbol {
                symbol: "$contract".into(),
            },
            self_type: self_type(&base),
            type_parameters: vec![],
            methods: vec![],
            associated_types: vec![members[1].clone()],
        },
    )
    .unwrap();
    let DeclarationPayload::OwnedContract(contract) = value(&lowerer, 1) else {
        panic!("contract")
    };
    assert_eq!(contract.associated_types.len(), 1);
    assert_eq!(contract.associated_types[0].id, members[1].id);
    lower(
        &mut lowerer,
        &AuthoredChange::SetOwnedImplementation {
            declaration: DeclarationSelector::Symbol {
                symbol: "$implementation".into(),
            },
            contract: AuthoredDeclarationReference::Local {
                declaration: DeclarationSelector::Symbol {
                    symbol: "$contract".into(),
                },
            },
            type_parameters: vec![],
            implementation_parameters: vec![],
            self_type: AuthoredType::OwnedI64Cell {},
            type_arguments: vec![],
            methods: vec![],
            associated_types: vec![bindings[1].clone()],
        },
    )
    .unwrap();
    let DeclarationPayload::OwnedImplementation(implementation) = value(&lowerer, 2) else {
        panic!("implementation")
    };
    assert_eq!(implementation.associated_types.len(), 1);
    assert_eq!(implementation.associated_types[0].member, members[1].id);
    // Explicitly empty Set removes feature data rather than preserving a hidden inventory.
    lower(
        &mut lowerer,
        &AuthoredChange::SetOwnedImplementation {
            declaration: DeclarationSelector::Symbol {
                symbol: "$implementation".into(),
            },
            contract: AuthoredDeclarationReference::Local {
                declaration: DeclarationSelector::Symbol {
                    symbol: "$contract".into(),
                },
            },
            type_parameters: vec![],
            implementation_parameters: vec![],
            self_type: AuthoredType::OwnedI64Cell {},
            type_arguments: vec![],
            methods: vec![],
            associated_types: vec![],
        },
    )
    .unwrap();
    let DeclarationPayload::OwnedImplementation(implementation) = value(&lowerer, 2) else {
        panic!("implementation")
    };
    assert!(implementation.associated_types.is_empty());
}

#[test]
fn associated_lowering_rejects_duplicate_erasure_and_admits_cumulative_work_before_rhs_growth() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let witness = crate::platform::witness::rebuild_full_witness(&base).unwrap();
    let members = members();
    for same_name in [false, true] {
        let mut duplicate = members.clone();
        if same_name {
            duplicate[1].name = duplicate[0].name.clone();
        } else {
            duplicate[1].id = duplicate[0].id;
        }
        let mut lowerer = lowerer(&base, &witness, ChangeBudget::default());
        assert_eq!(
            lowerer
                .owned_contract_value(&self_type(&base), &[], &[], &duplicate)
                .unwrap_err()
                .code,
            "change_authored_associated_member_duplicate"
        );
    }
    let contract = AuthoredDeclarationReference::Exact {
        package: PackageId::migrate(b"associated-foreign", 1),
        declaration: DeclarationId::migrate(b"associated-foreign", 1),
    };
    let bindings = vec![
        AuthoredAssociatedTypeBinding {
            member: members[0].id,
            value: AuthoredAssociatedTypeBindingValue::Owned {
                ty: AuthoredType::OwnedI64Cell {},
            },
        },
        AuthoredAssociatedTypeBinding {
            member: members[1].id,
            value: AuthoredAssociatedTypeBindingValue::Owned {
                ty: AuthoredType::ByteBuffer {},
            },
        },
    ];
    for maximum in [None, Some(0), Some(11), Some(12), Some(u64::MAX)] {
        let mut budget = ChangeBudget::default();
        budget.validation.maximum_expression_steps = maximum;
        let mut lowerer = lowerer(&base, &witness, budget);
        let contract_result = lowerer.owned_contract_value(&self_type(&base), &[], &[], &members);
        if maximum == Some(0) {
            assert_eq!(
                contract_result.unwrap_err().code,
                "change_budget_validation_expression_steps"
            );
            assert_eq!(lowerer.work.associated_inventory_steps, 0);
            assert!(!lowerer.work.associated_inventory_saturated);
            assert_eq!(lowerer.types.len(), 0);
            continue;
        }
        contract_result.unwrap();
        let result = lowerer.owned_implementation_value(
            &contract,
            &[],
            &[],
            &AuthoredType::OwnedI64Cell {},
            &[],
            &[],
            &bindings,
        );
        if maximum == Some(11) {
            assert_eq!(
                result.unwrap_err().code,
                "change_budget_validation_expression_steps"
            );
            assert_eq!(lowerer.types.len(), 0);
            assert_eq!(lowerer.work.associated_inventory_steps, 6);
        } else {
            assert_eq!(result.unwrap().associated_types.len(), 2);
            assert_eq!(lowerer.work.associated_inventory_steps, 12);
        }
        assert!(!lowerer.work.associated_inventory_saturated);
    }
    let mut duplicate = bindings.clone();
    duplicate[1].member = duplicate[0].member;
    let mut lowerer = lowerer(&base, &witness, ChangeBudget::default());
    assert_eq!(
        lowerer
            .owned_implementation_value(
                &contract,
                &[],
                &[],
                &AuthoredType::OwnedI64Cell {},
                &[],
                &[],
                &duplicate
            )
            .unwrap_err()
            .code,
        "change_authored_associated_binding_duplicate"
    );
    let canonical_contract = DeclarationReference {
        package: PackageId::migrate(b"associated-foreign", 1),
        declaration: DeclarationId::migrate(b"associated-foreign", 1),
    };
    let canonical_scope = DeclarationReference {
        package: PackageId::migrate(b"associated-foreign", 2),
        declaration: DeclarationId::migrate(b"associated-foreign", 2),
    };
    let parameter = ImplementationParameterId::migrate(b"associated-foreign", 3);
    let forms = [
        (
            AuthoredType::AssociatedContractMember {
                contract: contract.clone(),
                member: members[0].id,
            },
            TypeForm::AssociatedContractMember {
                contract: canonical_contract,
                member: members[0].id,
            },
        ),
        (
            AuthoredType::AssociatedWitnessProjection {
                scope: AuthoredDeclarationReference::Exact {
                    package: canonical_scope.package,
                    declaration: canonical_scope.declaration,
                },
                parameter,
                contract,
                member: members[1].id,
            },
            TypeForm::AssociatedWitnessProjection {
                scope: canonical_scope,
                parameter,
                contract: canonical_contract,
                member: members[1].id,
            },
        ),
    ];
    for (authored, expected) in forms {
        let digest = lowerer.lower_type(&authored).unwrap();
        let object = lowerer.types.get(digest).unwrap();
        assert_eq!(object.form, expected);
        let (encoded_digest, bytes) = crate::platform::kernel::encode_type_object(object).unwrap();
        assert_eq!(encoded_digest, digest);
        assert_eq!(
            crate::platform::kernel::decode_type_object(&bytes, digest)
                .unwrap()
                .form,
            expected
        );
    }
}

#[test]
fn associated_inventory_cancellation_precedes_retained_owner_and_type_growth() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let witness = crate::platform::witness::rebuild_full_witness(&base).unwrap();
    let members = members();
    let bindings = vec![
        AuthoredAssociatedTypeBinding {
            member: members[0].id,
            value: AuthoredAssociatedTypeBindingValue::Owned {
                ty: AuthoredType::OwnedI64Cell {},
            },
        },
        AuthoredAssociatedTypeBinding {
            member: members[1].id,
            value: AuthoredAssociatedTypeBindingValue::Owned {
                ty: AuthoredType::ByteBuffer {},
            },
        },
    ];
    let contract = AuthoredDeclarationReference::Exact {
        package: PackageId::migrate(b"associated-foreign", 1),
        declaration: DeclarationId::migrate(b"associated-foreign", 1),
    };
    for implementation in [false, true] {
        for checks in [0, 1] {
            let controlled = ControlledBase {
                base: &base,
                control: ExecutionControl::cancel_after_checks(checks),
            };
            let mut lowerer = AuthoredLowerer::new(
                &controlled,
                &witness,
                AuthoredLoweringInputs {
                    intent_base: RevisionId::from_digest([9; 32]),
                    allocation_seed: [7; 32],
                    deletion_change: ChangeDigest::of(b"cancelled associated inventory"),
                    allocated: std::collections::BTreeMap::new(),
                    definitions: std::collections::BTreeMap::new(),
                    allocations: vec![],
                    budget: ChangeBudget::default(),
                },
            )
            .unwrap();
            let error = if implementation {
                lowerer
                    .owned_implementation_value(
                        &contract,
                        &[],
                        &[],
                        &AuthoredType::OwnedI64Cell {},
                        &[],
                        &[],
                        &bindings,
                    )
                    .unwrap_err()
            } else {
                lowerer
                    .owned_contract_value(&self_type(&base), &[], &[], &members)
                    .unwrap_err()
            };
            assert_eq!(error.class, DiagnosticClass::Cancelled);
            assert!(lowerer.owners.is_empty());
            assert!(lowerer.owner_edits.is_empty());
            assert_eq!(lowerer.types.len(), 0);
            assert!(lowerer.type_additions.is_empty());
            assert_eq!(
                lowerer.work.associated_inventory_steps,
                if checks == 0 { 0 } else { 6 }
            );
            assert_eq!(
                lowerer.work.canonical,
                crate::platform::change::CanonicalReadWork::default()
            );
        }
    }
}
