//! Direct table-helper authority tests. These raw point-reader records are not
//! admitted packages, compiler artifacts, or evidence that a loader gate is open.
use super::{admit_work, contains_associated_types, validate_unit_types};
use crate::platform::compiler::unit::{
    BYTECODE_CONTRACT_VERSION, COMPILER_UNIT_CONTRACT_VERSION, CompilationPayload,
    CompilationSource, CompilationTables, CompilationUnit, CompilationUnitKey, CompiledCode,
    CompiledInstruction, CompiledSignature, OptimizationPolicy,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::owned_associated::{
    AssociatedAdmissionLimits, OwnedAssociatedAdmission,
};
use crate::platform::kernel::{
    AssociatedTypeBound, AssociatedTypeMember, CallableResult, DeclarationPayload,
    DeclarationRecord, DeclarationReference, DeclarationVisibility, DependencyRecord,
    ExpressionRead, FunctionDeclaration, FunctionEffect, ImplementationParameter, Name,
    OwnedContract, OwnedMethod, OwnedMethodParameter, OwnerHeader, OwnerKey, OwnerKind,
    OwnerRecord, PackageId, PackageInterfaceDeclaration, PackageInterfaceDeclarationPayload,
    PackageInterfaceRecord, PackageRevisionDigest, ParameterUse, ResultRelation,
    StructuralTypeField, TypeForm, TypeObject, TypeObjectDigest, TypeParameterConstraints,
    TypeParameterRecord, encode_type_object,
};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, DeclarationId, ExpressionId, ImplementationParameterId, MethodId,
    ModuleId, RevisionId, TypeParameterId,
};
use crate::platform::witness::SemanticDigest;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

const SEED: &[u8] = b"artifact-associated-point-table";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReadEvent {
    Owner(OwnerKey),
    Type(TypeObjectDigest),
}

struct PointRead {
    package: PackageId,
    owners: BTreeMap<OwnerKey, OwnerRecord>,
    types: BTreeMap<TypeObjectDigest, TypeObject>,
    dependencies: BTreeMap<PackageId, DependencyRecord>,
    imports: BTreeMap<(PackageId, OwnerKey), PackageInterfaceRecord>,
    representation_dependencies: BTreeMap<PackageId, DependencyRecord>,
    representations: BTreeMap<(PackageId, OwnerKey), PackageInterfaceRecord>,
    control: ExecutionControl,
    checks: Cell<u64>,
    reads: RefCell<Vec<ReadEvent>>,
}

impl PointRead {
    fn checkpoint(&self) -> Result<(), Diagnostic> {
        self.checks.set(self.checks.get() + 1);
        self.control
            .check()
            .map_err(crate::platform::execution::normalized::execution_diagnostic)
    }

    fn reset_observation(&mut self, control: ExecutionControl) {
        self.control = control;
        self.checks.set(0);
        self.reads.get_mut().clear();
    }

    fn add_type(&mut self, form: TypeForm) -> TypeObjectDigest {
        let object = TypeObject::new(form).unwrap();
        let digest = encode_type_object(&object).unwrap().0;
        self.types.insert(digest, object);
        digest
    }

    fn add_declaration(
        &mut self,
        ordinal: u64,
        label: &str,
        kind: OwnerKind,
        payload: DeclarationPayload,
    ) -> DeclarationReference {
        let id = DeclarationId::migrate(SEED, ordinal);
        self.owners.insert(
            OwnerKey::Declaration(id),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(OwnerKey::Declaration(id), kind),
                module: ModuleId::migrate(SEED, 0),
                name: Name::new(label).unwrap(),
                visibility: DeclarationVisibility::Public,
                payload,
            }),
        );
        DeclarationReference {
            package: self.package,
            declaration: id,
        }
    }

    fn add_parameter(
        &mut self,
        declaration: DeclarationReference,
        ordinal: u64,
    ) -> (TypeParameterId, TypeObjectDigest) {
        let parameter = TypeParameterId::migrate(SEED, ordinal);
        self.owners.insert(
            OwnerKey::TypeParameter(parameter),
            OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(parameter),
                    OwnerKind::TypeParameter,
                ),
                declaration: declaration.declaration,
                name: Name::new(format!("P{ordinal}")).unwrap(),
                constraints: TypeParameterConstraints::Owned,
            }),
        );
        let ty = self.add_type(TypeForm::TypeParameter { parameter });
        (parameter, ty)
    }
}

impl ExpressionRead for PointRead {
    fn package_id(&self) -> PackageId {
        self.package
    }

    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.checkpoint()?;
        self.reads.borrow_mut().push(ReadEvent::Owner(owner));
        Ok(self.owners.get(&owner).cloned())
    }

    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        self.checkpoint()?;
        self.reads.borrow_mut().push(ReadEvent::Type(digest));
        Ok(self.types.get(&digest).cloned())
    }

    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.checkpoint()?;
        Ok(self.imports.get(&(package, owner)).cloned())
    }

    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.checkpoint()?;
        // Selection presence supplies naming authority; the record's generation
        // describes its encoding, not the selected declaration's generation.
        Ok(self.dependencies.contains_key(&package))
    }

    fn package_representation_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.checkpoint()?;
        Ok(self.representations.get(&(package, owner)).cloned())
    }

    fn has_representation_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.checkpoint()?;
        Ok(self.representation_dependencies.contains_key(&package))
    }

    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.checkpoint()
    }
}

struct Fixture {
    read: PointRead,
    ordinary: DeclarationReference,
    lexical: DeclarationReference,
    contract: DeclarationReference,
    marker: DeclarationReference,
    member: AssociatedTypeMemberId,
    witness: ImplementationParameterId,
    unit_type: TypeObjectDigest,
    projection: TypeObjectDigest,
    abstract_member: TypeObjectDigest,
}

fn empty_function(result: TypeObjectDigest) -> FunctionDeclaration {
    FunctionDeclaration {
        type_parameters: vec![],
        implementation_parameters: vec![],
        requirement_parameters: vec![],
        effect_parameters: vec![],
        parameters: vec![],
        result: CallableResult {
            ty: result,
            relation: ResultRelation::Value,
        },
        read_scope_parameters: vec![],
        read_scope_sources: vec![],
        effect: FunctionEffect::Pure,
        body: ExpressionId::migrate(SEED, 0),
    }
}

fn fixture() -> Fixture {
    let mut read = PointRead {
        package: PackageId::from_bytes([0xc7; 16]).unwrap(),
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        imports: BTreeMap::new(),
        representation_dependencies: BTreeMap::new(),
        representations: BTreeMap::new(),
        control: ExecutionControl::uncancelled(),
        checks: Cell::new(0),
        reads: RefCell::new(vec![]),
    };
    let unit_type = read.add_type(TypeForm::Unit);
    let ordinary = read.add_declaration(
        1,
        "ordinary",
        OwnerKind::PureFunction,
        DeclarationPayload::Function(empty_function(unit_type)),
    );
    let contract = DeclarationReference {
        package: read.package,
        declaration: DeclarationId::migrate(SEED, 2),
    };
    let (self_parameter, self_type) = read.add_parameter(contract, 2);
    let member = AssociatedTypeMemberId::migrate(SEED, 0);
    let abstract_member = read.add_type(TypeForm::AssociatedContractMember { contract, member });
    assert_eq!(
        read.add_declaration(
            2,
            "Reader",
            OwnerKind::OwnedContract,
            DeclarationPayload::OwnedContract(OwnedContract {
                self_parameter,
                type_parameters: vec![],
                associated_types: vec![AssociatedTypeMember {
                    id: member,
                    name: Name::new("Out").unwrap(),
                    bound: AssociatedTypeBound::Owned,
                }],
                methods: vec![OwnedMethod {
                    id: MethodId::migrate(SEED, 0),
                    name: Name::new("read").unwrap(),
                    parameters: vec![OwnedMethodParameter {
                        ty: self_type,
                        use_mode: ParameterUse::Borrow,
                    }],
                    result: CallableResult {
                        ty: abstract_member,
                        relation: ResultRelation::ReadFrom { source: 0 }
                    },
                    read_scope_parameters: vec![],
                    read_scope_sources: vec![],
                    effect: FunctionEffect::Pure,
                }],
            }),
        ),
        contract
    );
    let lexical = DeclarationReference {
        package: read.package,
        declaration: DeclarationId::migrate(SEED, 3),
    };
    let (formal, self_type) = read.add_parameter(lexical, 3);
    let witness = ImplementationParameterId::migrate(SEED, 0);
    let mut function = empty_function(unit_type);
    function.type_parameters.push(formal);
    function
        .implementation_parameters
        .push(ImplementationParameter {
            id: witness,
            name: Name::new("reader").unwrap(),
            contract,
            self_type,
            type_arguments: vec![],
        });
    assert_eq!(
        read.add_declaration(
            3,
            "lexical",
            OwnerKind::PureFunction,
            DeclarationPayload::Function(function),
        ),
        lexical
    );
    let projection = read.add_type(TypeForm::AssociatedWitnessProjection {
        scope: lexical,
        parameter: witness,
        contract,
        member,
    });
    let marker = DeclarationReference {
        package: read.package,
        declaration: DeclarationId::migrate(SEED, 4),
    };
    let (formal, _) = read.add_parameter(marker, 4);
    assert_eq!(
        read.add_declaration(
            4,
            "Phantom",
            OwnerKind::OwnedRecord,
            DeclarationPayload::OwnedRecord {
                type_parameters: vec![formal],
                fields: vec![],
            },
        ),
        marker
    );
    Fixture {
        read,
        ordinary,
        lexical,
        contract,
        marker,
        member,
        witness,
        unit_type,
        projection,
        abstract_member,
    }
}

fn unit(f: &Fixture, scope: DeclarationReference, types: Vec<TypeObjectDigest>) -> CompilationUnit {
    let key = OwnerKey::Declaration(scope.declaration);
    let OwnerRecord::Declaration(owner) = &f.read.owners[&key] else {
        unreachable!()
    };
    // Summary dimensions and code are irrelevant to this isolated table helper;
    // no encoded unit, loader correspondence, or FullWitness is manufactured.
    let dimension = SemanticDigest::from_bytes([7; 32]);
    let source = CompilationSource {
        package: scope.package,
        owner: key,
        kind: owner.header.kind,
        semantic_interface: dimension,
        implementation: dimension,
        type_digest: dimension,
        effect: dimension,
        capability: dimension,
        test: None,
        validation_dependencies: dimension,
    };
    let payload = match &owner.payload {
        DeclarationPayload::Function(function) => CompilationPayload::Function {
            signature: CompiledSignature {
                implementation_parameters: function.implementation_parameters.clone(),
                requirement_parameters: vec![],
                effect_parameters: vec![],
                effect: function.effect.clone(),
                type_parameters: function.type_parameters.clone(),
                type_parameter_constraints: vec![
                    TypeParameterConstraints::Owned;
                    function.type_parameters.len()
                ],
                parameters: vec![],
                result: CallableResult {
                    ty: types.iter().position(|ty| *ty == f.unit_type).unwrap() as u32,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: vec![],
                read_scope_sources: vec![],
                task_requirements: vec![],
            },
            code: CompiledCode {
                parameter_count: 0,
                local_count: 0,
                instructions: vec![CompiledInstruction::Unit, CompiledInstruction::Return],
            },
        },
        DeclarationPayload::OwnedContract(contract) => {
            CompilationPayload::OwnedContract(contract.clone())
        }
        DeclarationPayload::OwnedImplementation(implementation) => {
            CompilationPayload::OwnedImplementation(implementation.clone())
        }
        _ => unreachable!(),
    };
    let optimization = OptimizationPolicy::DeterministicBaseline;
    CompilationUnit {
        contract_version: COMPILER_UNIT_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        bytecode_contract_version: BYTECODE_CONTRACT_VERSION,
        key: CompilationUnitKey::derive_for_tag(
            &source,
            optimization,
            match &payload {
                CompilationPayload::OwnedContract(_) => Some(1),
                CompilationPayload::OwnedImplementation(_) => Some(2),
                _ => None,
            },
        )
        .unwrap(),
        source,
        optimization,
        tables: CompilationTables {
            declarations: vec![],
            fields: vec![],
            read_view_fields: vec![],
            cases: vec![],
            requirements: vec![],
            operations: vec![],
            ports: vec![],
            types,
            structural_names: vec![],
            texts: vec![],
        },
        payload,
    }
}

fn assert_bad_table(f: &Fixture, unit: &CompilationUnit) {
    let error = validate_unit_types(&f.read, unit, &OwnedAssociatedAdmission::new()).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "artifact_associated_type");
}

#[test]
fn artifact_associated_table_helper_accepts_exact_lexical_contract_and_ordinary_contexts() {
    let f = fixture();
    for unit in [
        unit(&f, f.ordinary, vec![f.unit_type]),
        unit(&f, f.lexical, vec![f.unit_type, f.projection]),
        unit(&f, f.contract, vec![f.abstract_member]),
    ] {
        validate_unit_types(&f.read, &unit, &OwnedAssociatedAdmission::new()).unwrap();
    }
}

#[test]
fn artifact_associated_table_helper_rejects_unused_foreign_and_nonexistent_projections() {
    let mut f = fixture();
    let ordinary = unit(&f, f.ordinary, vec![f.unit_type]);
    validate_unit_types(&f.read, &ordinary, &OwnedAssociatedAdmission::new()).unwrap();
    for (scope, parameter, member) in [
        (f.lexical, f.witness, f.member),
        (
            DeclarationReference {
                package: PackageId::from_bytes([0x18; 16]).unwrap(),
                declaration: f.ordinary.declaration,
            },
            f.witness,
            f.member,
        ),
        (
            f.ordinary,
            ImplementationParameterId::migrate(SEED, 99),
            f.member,
        ),
    ] {
        let projection = f.read.add_type(TypeForm::AssociatedWitnessProjection {
            scope,
            parameter,
            contract: f.contract,
            member,
        });
        let mut mutant = ordinary.clone();
        mutant.tables.types.push(projection);
        // Its result and bytecode still use only Unit. Unused table roots count.
        assert_bad_table(&f, &mutant);
    }
    let missing = f.read.add_type(TypeForm::AssociatedWitnessProjection {
        scope: f.lexical,
        parameter: f.witness,
        contract: f.contract,
        member: AssociatedTypeMemberId::migrate(SEED, 99),
    });
    assert_bad_table(&f, &unit(&f, f.lexical, vec![f.unit_type, missing]));
}

#[test]
fn artifact_associated_table_helper_uses_source_binding_and_cannot_reuse_another_owner_scope() {
    let mut f = fixture();
    let original = unit(&f, f.lexical, vec![f.unit_type, f.projection]);
    let shared = OwnedAssociatedAdmission::new();
    validate_unit_types(&f.read, &original, &shared).unwrap();
    let swapped = unit(&f, f.ordinary, original.tables.types.clone());
    let error = validate_unit_types(&f.read, &swapped, &shared).unwrap_err();
    assert_eq!(error.code, "artifact_associated_type");
    for mutation in 0..4 {
        let mut mutant = original.clone();
        match mutation {
            0 => mutant.source.package = PackageId::from_bytes([0x29; 16]).unwrap(),
            1 => {
                mutant.source.owner = OwnerKey::Declaration(DeclarationId::migrate(SEED, 99));
            }
            2 => mutant.source.kind = OwnerKind::TaskFunction,
            3 => {
                let OwnerRecord::Declaration(record) =
                    f.read.owners.get_mut(&original.source.owner).unwrap()
                else {
                    unreachable!()
                };
                record.header.owner = OwnerKey::Declaration(f.ordinary.declaration);
            }
            _ => unreachable!(),
        }
        let error =
            validate_unit_types(&f.read, &mutant, &OwnedAssociatedAdmission::new()).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Corrupt);
        assert_eq!(error.code, "artifact_associated_owner");
    }
}

#[test]
fn artifact_associated_table_helper_current_unit_does_not_upgrade_canonical_owner_generation() {
    for scope_contract in [false, true] {
        let mut f = fixture();
        let scope = if scope_contract {
            f.contract
        } else {
            f.lexical
        };
        let roots = if scope_contract {
            vec![f.abstract_member]
        } else {
            vec![f.unit_type, f.projection]
        };
        let unit = unit(&f, scope, roots);
        assert_eq!(unit.graph_contract_version, 34);
        validate_unit_types(&f.read, &unit, &OwnedAssociatedAdmission::new()).unwrap();
        f.read
            .owners
            .get_mut(&unit.source.owner)
            .unwrap()
            .set_encoding_for_edit(32);
        assert_bad_table(&f, &unit);
    }
    let mut f = fixture();
    let ordinary = unit(&f, f.ordinary, vec![f.unit_type]);
    f.read
        .owners
        .get_mut(&ordinary.source.owner)
        .unwrap()
        .set_encoding_for_edit(32);
    validate_unit_types(&f.read, &ordinary, &OwnedAssociatedAdmission::new()).unwrap();
}

#[test]
fn artifact_associated_table_helper_walks_nested_and_phantom_unused_actuals() {
    let mut f = fixture();
    let phantom = f.read.add_type(TypeForm::Applied {
        declaration: f.marker,
        arguments: vec![f.projection],
    });
    let nested = f.read.add_type(TypeForm::OwnedProduct {
        fields: vec![StructuralTypeField {
            name: Name::new("hidden").unwrap(),
            ty: phantom,
        }],
    });
    validate_unit_types(
        &f.read,
        &unit(&f, f.lexical, vec![f.unit_type, nested]),
        &OwnedAssociatedAdmission::new(),
    )
    .unwrap();
    let missing = f.read.add_type(TypeForm::AssociatedWitnessProjection {
        scope: f.lexical,
        parameter: ImplementationParameterId::migrate(SEED, 99),
        contract: f.contract,
        member: f.member,
    });
    let phantom = f.read.add_type(TypeForm::Applied {
        declaration: f.marker,
        arguments: vec![missing],
    });
    let nested = f.read.add_type(TypeForm::OwnedSequence { item: phantom });
    for root in [phantom, nested] {
        assert_bad_table(&f, &unit(&f, f.lexical, vec![f.unit_type, root]));
    }
}

#[test]
fn artifact_aggregate_table_admission_keeps_each_inline_method_scope_exact() {
    use crate::platform::kernel::{
        ReadScopeArgument, ReadScopeBinder, ReadScopeParameter, ReadScopeReference, ReadScopeSource,
    };
    use crate::platform::semantic_id::ReadScopeParameterId;
    let mut f = fixture();
    let member = AssociatedTypeMemberId::migrate(SEED, 90);
    // Equal formal IDs in distinct binders remain distinct scope coordinates.
    let scope_parameter = ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, 90),
        name: Name::new("read_scope").unwrap(),
    };
    let methods = [MethodId::migrate(SEED, 90), MethodId::migrate(SEED, 91)];
    let scopes = methods.map(|method| {
        ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::ContractMethod {
                contract: f.contract,
                method,
            },
            parameter: scope_parameter.id,
        })
    });
    let views = scopes.map(|read_scope| {
        f.read.add_type(TypeForm::AssociatedReadViewMember {
            contract: f.contract,
            member,
            read_scope,
        })
    });
    let key = OwnerKey::Declaration(f.contract.declaration);
    let self_type;
    {
        let OwnerRecord::Declaration(record) = f.read.owners.get_mut(&key).unwrap() else {
            unreachable!()
        };
        let DeclarationPayload::OwnedContract(contract) = &mut record.payload else {
            unreachable!()
        };
        self_type = contract.methods[0].parameters[0].ty;
        contract.associated_types.push(AssociatedTypeMember {
            id: member,
            name: Name::new("View").unwrap(),
            bound: AssociatedTypeBound::ReadViewFamily {
                scope_parameter: scope_parameter.clone(),
            },
        });
        for index in 0..2 {
            contract.methods.push(OwnedMethod {
                id: methods[index],
                name: Name::new(format!("view_{index}")).unwrap(),
                parameters: vec![OwnedMethodParameter {
                    ty: self_type,
                    use_mode: ParameterUse::Borrow,
                }],
                result: CallableResult {
                    ty: views[index],
                    relation: ResultRelation::ViewFrom {
                        source: 0,
                        scope: scopes[index],
                    },
                },
                read_scope_parameters: vec![scope_parameter.clone()],
                read_scope_sources: vec![ReadScopeSource {
                    source: 0,
                    scope: scopes[index],
                }],
                effect: FunctionEffect::Pure,
            });
        }
    }
    let current = unit(
        &f,
        f.contract,
        vec![self_type, f.abstract_member, views[0], views[1]],
    );
    let admission = OwnedAssociatedAdmission::new();
    validate_unit_types(&f.read, &current, &admission).unwrap();
    {
        let OwnerRecord::Declaration(record) = f.read.owners.get_mut(&key).unwrap() else {
            unreachable!()
        };
        let DeclarationPayload::OwnedContract(contract) = &mut record.payload else {
            unreachable!()
        };
        // Both roots occur in the table, but the first method's scope cannot
        // certify the second method's root. Aggregate admission checks each
        // canonical occurrence before considering complete unused table entries.
        contract.methods.last_mut().unwrap().result.ty = views[0];
    }
    let error = validate_unit_types(&f.read, &current, &admission).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "artifact_associated_type");
}

#[test]
fn artifact_aggregate_table_admission_keeps_each_family_binding_scope_exact() {
    use crate::platform::kernel::{
        AssociatedTypeBinding, AssociatedTypeBindingValue, OwnedImplementation,
        OwnedMethodImplementation, ReadScopeArgument, ReadScopeBinder, ReadScopeParameter,
        ReadScopeReference, ReadViewDeclaration,
    };
    use crate::platform::semantic_id::ReadScopeParameterId;
    let mut f = fixture();
    let members = [
        AssociatedTypeMemberId::migrate(SEED, 92),
        AssociatedTypeMemberId::migrate(SEED, 93),
    ];
    let scope_parameter = ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, 92),
        name: Name::new("read_scope").unwrap(),
    };
    let view = f.read.add_declaration(
        92,
        "BindingView",
        OwnerKind::ReadView,
        DeclarationPayload::ReadView(ReadViewDeclaration {
            type_parameters: vec![],
            scope_parameter: scope_parameter.clone(),
            fields: vec![],
        }),
    );
    let implementation = DeclarationReference {
        package: f.read.package,
        declaration: DeclarationId::migrate(SEED, 93),
    };
    let templates = members.map(|member| {
        f.read.add_type(TypeForm::ReadViewApplied {
            declaration: view,
            arguments: vec![],
            read_scope: ReadScopeArgument::Parameter(ReadScopeReference {
                binder: ReadScopeBinder::AssociatedBinding {
                    implementation,
                    member,
                },
                parameter: scope_parameter.id,
            }),
        })
    });
    {
        let OwnerRecord::Declaration(record) = f
            .read
            .owners
            .get_mut(&OwnerKey::Declaration(f.contract.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::OwnedContract(contract) = &mut record.payload else {
            unreachable!()
        };
        // Keep a valid complete unused method inventory while isolating the
        // two family bindings. No empty-method malformed control is admitted.
        contract.methods = vec![OwnedMethod {
            id: MethodId::migrate(SEED, 92),
            name: Name::new("unused").unwrap(),
            parameters: vec![],
            result: CallableResult {
                ty: f.unit_type,
                relation: ResultRelation::Value,
            },
            read_scope_parameters: vec![],
            read_scope_sources: vec![],
            effect: FunctionEffect::Pure,
        }];
        contract.associated_types = members
            .into_iter()
            .enumerate()
            .map(|(index, id)| AssociatedTypeMember {
                id,
                name: Name::new(format!("View_{index}")).unwrap(),
                bound: AssociatedTypeBound::ReadViewFamily {
                    scope_parameter: scope_parameter.clone(),
                },
            })
            .collect();
    }
    let self_type = f.read.add_type(TypeForm::ByteBuffer);
    let mut bindings = members
        .into_iter()
        .zip(templates)
        .map(|(member, ty)| AssociatedTypeBinding {
            member,
            value: AssociatedTypeBindingValue::ReadViewFamily {
                scope_parameter: scope_parameter.clone(),
                ty,
            },
        })
        .collect::<Vec<_>>();
    bindings.sort_by_key(|binding| binding.member);
    assert_eq!(
        f.read.add_declaration(
            93,
            "ViewSupplier",
            OwnerKind::OwnedImplementation,
            DeclarationPayload::OwnedImplementation(OwnedImplementation {
                type_parameters: vec![],
                implementation_parameters: vec![],
                contract: f.contract,
                self_type,
                type_arguments: vec![],
                methods: vec![OwnedMethodImplementation {
                    method: MethodId::migrate(SEED, 92),
                    function: f.ordinary,
                    type_arguments: vec![],
                    implementations: vec![],
                    read_scope_arguments: vec![],
                }],
                associated_types: bindings,
            }),
        ),
        implementation
    );
    let current = unit(
        &f,
        implementation,
        vec![self_type, templates[0], templates[1]],
    );
    let admission = OwnedAssociatedAdmission::new();
    validate_unit_types(&f.read, &current, &admission).unwrap();
    {
        let OwnerRecord::Declaration(record) =
            f.read.owners.get_mut(&current.source.owner).unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(body) = &mut record.payload else {
            unreachable!()
        };
        let second = body
            .associated_types
            .iter_mut()
            .find(|binding| binding.member == members[1])
            .unwrap();
        let AssociatedTypeBindingValue::ReadViewFamily { ty, .. } = &mut second.value else {
            unreachable!()
        };
        // The template exists and is otherwise valid; its exact binding owner is
        // the first member. Equal formal IDs cannot certify the second binding.
        *ty = templates[0];
    }
    let error = validate_unit_types(&f.read, &current, &admission).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "artifact_associated_type");
}

#[test]
fn artifact_associated_table_helper_accepts_older_edge_encoding_for_current_foreign_owner() {
    let foreign = PackageId::from_bytes([0x35; 16]).unwrap();
    let dependency = DependencyRecord {
        graph_contract_version: 32,
        package: foreign,
        semantic_revision: RevisionId::from_digest([8; 32]),
        package_revision: PackageRevisionDigest::of(b"point-reader foreign metadata only"),
    };
    let mut f = fixture();
    let contract = DeclarationReference {
        package: foreign,
        declaration: DeclarationId::migrate(SEED, 50),
    };
    f.read.dependencies.insert(foreign, dependency.clone());
    let self_parameter = TypeParameterId::migrate(SEED, 50);
    f.read.imports.insert(
        (foreign, OwnerKey::TypeParameter(self_parameter)),
        PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(
                OwnerKey::TypeParameter(self_parameter),
                OwnerKind::TypeParameter,
            ),
            declaration: contract.declaration,
            name: Name::new("ForeignSelf").unwrap(),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    f.read.imports.insert(
        (foreign, OwnerKey::Declaration(contract.declaration)),
        PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
            header: OwnerHeader::new(
                OwnerKey::Declaration(contract.declaration),
                OwnerKind::OwnedContract,
            ),
            name: Name::new("ForeignReader").unwrap(),
            payload: PackageInterfaceDeclarationPayload::OwnedContract(OwnedContract {
                self_parameter,
                type_parameters: vec![],
                methods: vec![],
                associated_types: vec![AssociatedTypeMember {
                    id: f.member,
                    name: Name::new("Out").unwrap(),
                    bound: AssociatedTypeBound::Owned,
                }],
            }),
        }),
    );
    let OwnerRecord::Declaration(record) = f
        .read
        .owners
        .get_mut(&OwnerKey::Declaration(f.lexical.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        unreachable!()
    };
    function.implementation_parameters[0].contract = contract;
    let projection = f.read.add_type(TypeForm::AssociatedWitnessProjection {
        scope: f.lexical,
        parameter: f.witness,
        contract,
        member: f.member,
    });
    let foreign_unit = unit(&f, f.lexical, vec![f.unit_type, projection]);
    // The selected foreign contract and Self formal are current declarations,
    // even though the exact dependency record uses a supported older encoding.
    validate_unit_types(&f.read, &foreign_unit, &OwnedAssociatedAdmission::new()).unwrap();
    f.read
        .representation_dependencies
        .insert(foreign, dependency.clone());
    f.read.dependencies.remove(&foreign).unwrap();
    // The imported names remain present, but a representation-only selection
    // cannot authorize naming the foreign contract in an associated projection.
    assert_bad_table(&f, &foreign_unit);

    // Ordinary historical representation metadata remains usable alongside a
    // healthy local associated root; it supplies no foreign associated names.
    let mut f = fixture();
    f.read.dependencies.insert(foreign, dependency.clone());
    f.read
        .representation_dependencies
        .insert(foreign, dependency);
    let nominal = DeclarationReference {
        package: foreign,
        declaration: DeclarationId::migrate(SEED, 51),
    };
    let mut header = OwnerHeader::new(
        OwnerKey::Declaration(nominal.declaration),
        OwnerKind::Record,
    );
    header.contract_version = 32;
    f.read.representations.insert(
        (foreign, OwnerKey::Declaration(nominal.declaration)),
        PackageInterfaceRecord::RepresentationDeclaration(PackageInterfaceDeclaration {
            header,
            name: Name::new("HistoricalData").unwrap(),
            payload: PackageInterfaceDeclarationPayload::Record {
                type_parameters: vec![],
                fields: vec![],
            },
        }),
    );
    let ordinary = f.read.add_type(TypeForm::Named {
        declaration: nominal,
    });
    let mixed_unit = unit(&f, f.lexical, vec![f.unit_type, f.projection, ordinary]);
    validate_unit_types(&f.read, &mixed_unit, &OwnedAssociatedAdmission::new()).unwrap();
}

#[test]
fn artifact_associated_table_helper_measures_exact_cumulative_work_and_storage() {
    let f = fixture();
    let unit = unit(&f, f.lexical, vec![f.unit_type, f.projection]);
    let measured = OwnedAssociatedAdmission::new();
    validate_unit_types(&f.read, &unit, &measured).unwrap();
    let (steps, bytes) = measured.work();
    assert!(steps > 1 && bytes > 1);
    let limits = AssociatedAdmissionLimits {
        maximum_steps: steps,
        maximum_metadata_bytes: bytes,
        ..AssociatedAdmissionLimits::default()
    };
    let exact = OwnedAssociatedAdmission::with_limits(limits);
    validate_unit_types(&f.read, &unit, &exact).unwrap();
    assert_eq!(exact.work(), (steps, bytes));
    for limits in [
        AssociatedAdmissionLimits {
            maximum_steps: steps.checked_sub(1).unwrap(),
            ..limits
        },
        AssociatedAdmissionLimits {
            maximum_metadata_bytes: bytes.checked_sub(1).unwrap(),
            ..limits
        },
    ] {
        let admission = OwnedAssociatedAdmission::with_limits(limits);
        let error = validate_unit_types(&f.read, &unit, &admission).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Resource);
        assert!(admission.work().0 <= limits.maximum_steps);
        assert!(admission.work().1 <= limits.maximum_metadata_bytes);
    }
    // A fresh unit reader or repeated table hit must not reset the shared budget.
    let error = validate_unit_types(&f.read, &unit, &exact).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(exact.work(), (steps, bytes));
    f.read.reads.borrow_mut().clear();
    let empty = OwnedAssociatedAdmission::with_limits(AssociatedAdmissionLimits {
        maximum_steps: 0,
        ..AssociatedAdmissionLimits::default()
    });
    assert_eq!(
        validate_unit_types(&f.read, &unit, &empty)
            .unwrap_err()
            .class,
        DiagnosticClass::Resource
    );
    assert!(
        f.read
            .reads
            .borrow()
            .iter()
            .all(|event| matches!(event, ReadEvent::Owner(_)))
    );
    validate_unit_types(&f.read, &unit, &OwnedAssociatedAdmission::new()).unwrap();
}

#[test]
fn artifact_associated_table_helper_cancels_at_each_actual_checkpoint_and_retries() {
    let mut f = fixture();
    let unit = unit(&f, f.lexical, vec![f.unit_type, f.projection]);
    validate_unit_types(&f.read, &unit, &OwnedAssociatedAdmission::new()).unwrap();
    let checks = f.read.checks.get();
    let healthy_reads = f.read.reads.borrow().clone();
    assert!(checks > 1 && checks < 4096);
    for cutoff in 0..checks {
        f.read
            .reset_observation(ExecutionControl::cancel_after_checks(cutoff));
        let error =
            validate_unit_types(&f.read, &unit, &OwnedAssociatedAdmission::new()).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Cancelled, "cutoff={cutoff}");
        assert_eq!(error.code, "execution_cancelled");
        assert!(healthy_reads.starts_with(f.read.reads.borrow().as_slice()));
        if cutoff == 0 {
            assert!(f.read.reads.borrow().is_empty());
        }
    }
    f.read
        .reset_observation(ExecutionControl::cancel_after_checks(checks));
    validate_unit_types(&f.read, &unit, &OwnedAssociatedAdmission::new()).unwrap();
    assert_eq!(*f.read.reads.borrow(), healthy_reads);
    f.read.reset_observation(ExecutionControl::uncancelled());
    validate_unit_types(&f.read, &unit, &OwnedAssociatedAdmission::new()).unwrap();
    assert_eq!(f.read.checks.get(), checks);
}

#[test]
fn artifact_associated_discovery_inherits_work_and_refuses_before_the_next_object() {
    let mut f = fixture();
    let scalar = f.read.add_type(TypeForm::I64);
    let boolean = f.read.add_type(TypeForm::Bool);
    let ordinary: BTreeMap<_, _> = [f.unit_type, scalar, boolean]
        .into_iter()
        .map(|ty| (ty, f.read.types[&ty].clone()))
        .collect();
    let inherited = 17;
    let maximum = inherited + ordinary.len();
    let mut exact = inherited;
    assert!(!contains_associated_types(&ordinary, &mut exact, maximum).unwrap());
    assert_eq!(exact, maximum);
    let mut short = inherited;
    let error = contains_associated_types(&ordinary, &mut short, maximum - 1).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "artifact_associated_work");
    assert_eq!(short, maximum - 1);
    let associated = BTreeMap::from([(f.projection, f.read.types[&f.projection].clone())]);
    let mut work = maximum;
    assert!(contains_associated_types(&associated, &mut work, maximum + 1).unwrap());
    assert_eq!(work, maximum + 1);
    let mut short = maximum;
    assert_eq!(
        contains_associated_types(&associated, &mut short, maximum)
            .unwrap_err()
            .class,
        DiagnosticClass::Resource
    );
    assert_eq!(short, maximum);
    let mut ceiling = usize::MAX;
    assert_eq!(
        admit_work(&mut ceiling, usize::MAX).unwrap_err().class,
        DiagnosticClass::Resource
    );
    assert_eq!(ceiling, usize::MAX);
}
