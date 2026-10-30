//! Point-reader adversaries for associated authority; no normalized producer input.
use super::*;
use crate::platform::semantic_id::{
    DeclarationId, ExpressionId, FieldId, MethodId, ModuleId, ParameterId,
};

const SEED: &[u8] = b"owned-associated-admission-tests";
const GENERATION: u16 = crate::platform::canonical_schema::SCHEMA_EPOCH;
static EMPTY_READ_SCOPES: BTreeMap<ReadScopeReference, ReadScopeArgument> = BTreeMap::new();

struct Fixture {
    source: KernelSnapshot,
    contract: DeclarationReference,
    other_contract: DeclarationReference,
    scheme: DeclarationReference,
    concrete: DeclarationReference,
    lexical: DeclarationReference,
    other_lexical: DeclarationReference,
    external: DeclarationReference,
    nominal: DeclarationReference,
    member: AssociatedTypeMemberId,
    spare: AssociatedTypeMemberId,
    witness: ImplementationParameterId,
    scheme_parameter: TypeParameterId,
    cell: TypeObjectDigest,
    buffer: TypeObjectDigest,
}

fn name(value: &str) -> Name {
    Name::new(value).expect("canonical test name")
}

fn ty(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).expect("locally valid test type");
    let digest = encode_type_object(&object)
        .expect("canonical test type bytes")
        .0;
    source.types.insert(digest, object);
    digest
}

fn declaration(
    source: &mut KernelSnapshot,
    module: ModuleId,
    ordinal: u64,
    label: &str,
    kind: OwnerKind,
    payload: DeclarationPayload,
) -> DeclarationReference {
    let id = DeclarationId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::Declaration(id),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(id), kind),
            module,
            name: name(label),
            visibility: DeclarationVisibility::Public,
            payload,
        }),
    );
    DeclarationReference {
        package: source.root.package_id,
        declaration: id,
    }
}

fn parameter(
    source: &mut KernelSnapshot,
    scope: DeclarationReference,
    id: TypeParameterId,
) -> TypeObjectDigest {
    source.owners.insert(
        OwnerKey::TypeParameter(id),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(id), OwnerKind::TypeParameter),
            declaration: scope.declaration,
            name: name(&format!("P{}", id.bytes()[0])),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    ty(source, TypeForm::TypeParameter { parameter: id })
}

fn function(
    type_parameter: TypeParameterId,
    result: TypeObjectDigest,
    parameters: Vec<ImplementationParameter>,
) -> FunctionDeclaration {
    FunctionDeclaration {
        type_parameters: vec![type_parameter],
        implementation_parameters: parameters,
        requirement_parameters: vec![],
        effect_parameters: vec![],
        parameters: vec![],
        result: crate::platform::kernel::CallableResult {
            ty: result,
            relation: crate::platform::kernel::ResultRelation::Value,
        },
        effect: FunctionEffect::Pure,
        body: ExpressionId::migrate(SEED, 0),

        read_scope_parameters: Vec::new(),
        read_scope_sources: Vec::new(),
    }
}

fn fixture() -> Fixture {
    let mut source = super::super::tests::transport_snapshot();
    let module = source
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Module(record) => match record.header.owner {
                OwnerKey::Module(id) => Some(id),
                _ => None,
            },
            _ => None,
        })
        .expect("fixture has a module");
    let cell = ty(&mut source, TypeForm::OwnedI64Cell);
    let buffer = ty(&mut source, TypeForm::ByteBuffer);
    let scalar = ty(&mut source, TypeForm::I64);
    let member = AssociatedTypeMemberId::migrate(SEED, 0);
    let spare = AssociatedTypeMemberId::migrate(SEED, 1);
    let self_parameter = TypeParameterId::migrate(SEED, 0);
    let contract_value = OwnedContract {
        self_parameter,
        type_parameters: vec![],
        methods: vec![OwnedMethod {
            id: MethodId::migrate(SEED, 0),
            name: name("inspect"),
            parameters: vec![OwnedMethodParameter {
                ty: scalar,
                use_mode: ParameterUse::Unrestricted,
            }],
            result: crate::platform::kernel::CallableResult {
                ty: scalar,
                relation: crate::platform::kernel::ResultRelation::Value,
            },
            effect: FunctionEffect::Pure,

            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
        }],
        associated_types: vec![
            AssociatedTypeMember {
                id: member,
                name: name("Out"),
                bound: AssociatedTypeBound::Owned,
            },
            AssociatedTypeMember {
                id: spare,
                name: name("Unused"),
                bound: AssociatedTypeBound::Owned,
            },
        ],
    };
    let contract = declaration(
        &mut source,
        module,
        0,
        "Contract",
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(contract_value.clone()),
    );
    parameter(&mut source, contract, self_parameter);
    let mut other_contract_value = contract_value;
    other_contract_value.self_parameter = TypeParameterId::migrate(SEED, 40);
    let other_self_parameter = other_contract_value.self_parameter;
    let other_contract = declaration(
        &mut source,
        module,
        1,
        "OtherContract",
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(other_contract_value),
    );
    parameter(&mut source, other_contract, other_self_parameter);
    let scheme_parameter = TypeParameterId::migrate(SEED, 1);
    let mut bindings = vec![
        AssociatedTypeBinding {
            member,
            value: crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty: cell },
        },
        AssociatedTypeBinding {
            member: spare,
            value: crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty: cell },
        },
    ];
    bindings.sort_by_key(|binding| binding.member);
    let implementation = OwnedImplementation {
        type_parameters: vec![scheme_parameter],
        implementation_parameters: vec![],
        contract,
        self_type: cell,
        type_arguments: vec![],
        methods: vec![OwnedMethodImplementation {
            method: MethodId::migrate(SEED, 0),
            function: DeclarationReference {
                package: source.root.package_id,
                declaration: DeclarationId::migrate(SEED, 20),
            },
            type_arguments: vec![],
            implementations: vec![],

            read_scope_arguments: Vec::new(),
        }],
        associated_types: bindings,
    };
    let scheme = declaration(
        &mut source,
        module,
        2,
        "Scheme",
        OwnerKind::OwnedImplementation,
        DeclarationPayload::OwnedImplementation(implementation.clone()),
    );
    let symbolic = parameter(&mut source, scheme, scheme_parameter);
    if let Some(OwnerRecord::Declaration(record)) = source
        .owners
        .get_mut(&OwnerKey::Declaration(scheme.declaration))
        && let DeclarationPayload::OwnedImplementation(value) = &mut record.payload
    {
        value
            .associated_types
            .iter_mut()
            .find(|binding| binding.member == member)
            .expect("Out binding")
            .value = AssociatedTypeBindingValue::Owned { ty: symbolic };
    }
    let mut concrete_value = implementation;
    concrete_value.type_parameters.clear();
    concrete_value
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == member)
        .expect("Out binding")
        .value = AssociatedTypeBindingValue::Owned { ty: buffer };
    let concrete = declaration(
        &mut source,
        module,
        3,
        "Concrete",
        OwnerKind::OwnedImplementation,
        DeclarationPayload::OwnedImplementation(concrete_value),
    );
    let witness = ImplementationParameterId::migrate(SEED, 0);
    let lexical_parameter = TypeParameterId::migrate(SEED, 2);
    let lexical_ref = DeclarationReference {
        package: source.root.package_id,
        declaration: DeclarationId::migrate(SEED, 4),
    };
    let lexical_self = parameter(&mut source, lexical_ref, lexical_parameter);
    let lexical = declaration(
        &mut source,
        module,
        4,
        "Lexical",
        OwnerKind::PureFunction,
        DeclarationPayload::Function(function(
            lexical_parameter,
            lexical_self,
            vec![ImplementationParameter {
                id: witness,
                name: name("reader"),
                contract,
                self_type: lexical_self,
                type_arguments: vec![],
            }],
        )),
    );
    let other_parameter = TypeParameterId::migrate(SEED, 3);
    let other_ref = DeclarationReference {
        package: source.root.package_id,
        declaration: DeclarationId::migrate(SEED, 5),
    };
    let other_self = parameter(&mut source, other_ref, other_parameter);
    let other_lexical = declaration(
        &mut source,
        module,
        5,
        "OtherLexical",
        OwnerKind::PureFunction,
        DeclarationPayload::Function(function(
            other_parameter,
            other_self,
            vec![ImplementationParameter {
                id: witness,
                name: name("reader"),
                contract,
                self_type: other_self,
                type_arguments: vec![],
            }],
        )),
    );
    let external = declaration(
        &mut source,
        module,
        6,
        "External",
        OwnerKind::External,
        DeclarationPayload::External(ExternalDeclaration {
            type_parameters: vec![],
            parameters: vec![],
            result: scalar,
            implementation: ImplementationName::new("test.external").expect("implementation name"),
        }),
    );
    let field = FieldId::migrate(SEED, 0);
    let nominal_parameter = TypeParameterId::migrate(SEED, 4);
    let nominal = declaration(
        &mut source,
        module,
        7,
        "Nominal",
        OwnerKind::OwnedRecord,
        DeclarationPayload::OwnedRecord {
            type_parameters: vec![nominal_parameter],
            fields: vec![field],
        },
    );
    parameter(&mut source, nominal, nominal_parameter);
    source.owners.insert(
        OwnerKey::Field(field),
        OwnerRecord::Field(FieldRecord {
            header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
            declaration: nominal.declaration,
            name: name("value"),
            ty: cell,
        }),
    );
    let method_parameter = ParameterId::migrate(SEED, 0);
    let method_function = declaration(
        &mut source,
        module,
        20,
        "Inspect",
        OwnerKind::PureFunction,
        DeclarationPayload::Function(FunctionDeclaration {
            type_parameters: vec![],
            implementation_parameters: vec![],
            requirement_parameters: vec![],
            effect_parameters: vec![],
            parameters: vec![method_parameter],
            result: crate::platform::kernel::CallableResult {
                ty: scalar,
                relation: crate::platform::kernel::ResultRelation::Value,
            },
            effect: FunctionEffect::Pure,
            body: ExpressionId::migrate(SEED, 0),

            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
        }),
    );
    source.owners.insert(
        OwnerKey::Parameter(method_parameter),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(method_parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(method_function.declaration),
            name: name("value"),
            ty: scalar,
            use_mode: ParameterUse::Unrestricted,
            resource_requirement: None,
        }),
    );
    Fixture {
        source,
        contract,
        other_contract,
        scheme,
        concrete,
        lexical,
        other_lexical,
        external,
        nominal,
        member,
        spare,
        witness,
        scheme_parameter,
        cell,
        buffer,
    }
}

fn abstract_member(f: &Fixture) -> TypeForm {
    TypeForm::AssociatedContractMember {
        contract: f.contract,
        member: f.member,
    }
}
fn projection(f: &Fixture, scope: DeclarationReference) -> TypeForm {
    TypeForm::AssociatedWitnessProjection {
        scope,
        parameter: f.witness,
        contract: f.contract,
        member: f.member,
    }
}
fn implementation(f: &Fixture, reference: DeclarationReference) -> OwnedImplementation {
    owned_contract::implementation_record(&f.source, reference).expect("exact implementation")
}
fn replace_implementation(f: &mut Fixture, value: OwnedImplementation) {
    let Some(OwnerRecord::Declaration(record)) = f
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(f.scheme.declaration))
    else {
        panic!("implementation owner")
    };
    record.payload = DeclarationPayload::OwnedImplementation(value);
}
fn semantic(result: Result<(), Diagnostic>) {
    assert_eq!(
        result
            .expect_err("invalid associated authority must reject")
            .class,
        DiagnosticClass::Semantic
    );
}

#[test]
fn owned_associated_member_admission_is_per_occurrence_and_does_not_readmit_signatures() {
    let mut f = fixture();
    let form = abstract_member(&f);
    let root = ty(&mut f.source, form);
    let missing = TypeObjectDigest::from_bytes([249; 32]);
    let Some(OwnerRecord::Declaration(record)) = f
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(f.contract.declaration))
    else {
        panic!("contract")
    };
    let DeclarationPayload::OwnedContract(contract) = &mut record.payload else {
        panic!("contract payload")
    };
    contract.methods[0].result.ty = missing;
    let admission = OwnedAssociatedAdmission::new();
    admission
        .validate_occurrences(
            &f.source,
            &[root],
            AssociatedTypeContext::ContractSignature(f.contract),
            GENERATION,
        )
        .expect("inventory lookup must not recursively readmit method signatures");
    semantic(admission.validate_occurrences(
        &f.source,
        &[root],
        AssociatedTypeContext::Lexical(f.lexical),
        GENERATION,
    ));
    semantic(admission.validate_occurrences(
        &f.source,
        &[root],
        AssociatedTypeContext::ContractSignature(f.other_contract),
        GENERATION,
    ));
    semantic(admission.validate_occurrences(
        &f.source,
        &[root],
        AssociatedTypeContext::Forbidden,
        GENERATION,
    ));
}

#[test]
fn owned_associated_lexical_scope_contract_and_member_are_all_exact() {
    let f = fixture();
    let admission = OwnedAssociatedAdmission::new();
    admission
        .validate_form(
            &f.source,
            &projection(&f, f.lexical),
            AssociatedTypeContext::Lexical(f.lexical),
            GENERATION,
        )
        .expect("exact lexical projection");
    semantic(admission.validate_form(
        &f.source,
        &projection(&f, f.lexical),
        AssociatedTypeContext::Lexical(f.other_lexical),
        GENERATION,
    ));
    semantic(admission.validate_form(
        &f.source,
        &projection(&f, f.external),
        AssociatedTypeContext::Lexical(f.external),
        GENERATION,
    ));
    let foreign_contract = TypeForm::AssociatedWitnessProjection {
        scope: f.lexical,
        parameter: f.witness,
        contract: f.other_contract,
        member: f.member,
    };
    semantic(admission.validate_form(
        &f.source,
        &foreign_contract,
        AssociatedTypeContext::Lexical(f.lexical),
        GENERATION,
    ));
    let absent = TypeForm::AssociatedContractMember {
        contract: f.contract,
        member: AssociatedTypeMemberId::migrate(SEED, 99),
    };
    semantic(admission.validate_form(
        &f.source,
        &absent,
        AssociatedTypeContext::ContractSignature(f.contract),
        GENERATION,
    ));
}

#[test]
fn owned_associated_old_consuming_owners_and_scopes_do_not_inherit_generation() {
    let mut f = fixture();
    let admission = OwnedAssociatedAdmission::new();
    semantic(admission.validate_form(
        &f.source,
        &projection(&f, f.lexical),
        AssociatedTypeContext::Lexical(f.lexical),
        32,
    ));
    f.source
        .owners
        .get_mut(&OwnerKey::Declaration(f.lexical.declaration))
        .expect("scope owner")
        .set_encoding_for_edit(32);
    semantic(admission.validate_form(
        &f.source,
        &projection(&f, f.lexical),
        AssociatedTypeContext::Lexical(f.lexical),
        GENERATION,
    ));
}

#[test]
fn owned_associated_binding_inventory_and_unused_rhs_are_complete() {
    let mut f = fixture();
    let original = implementation(&f, f.scheme);
    OwnedAssociatedAdmission::new()
        .validate_bindings(&f.source, f.scheme, &original)
        .expect("complete symbolic bindings");
    let mut missing = original.clone();
    missing.associated_types.pop();
    replace_implementation(&mut f, missing.clone());
    semantic(OwnedAssociatedAdmission::new().validate_bindings(&f.source, f.scheme, &missing));
    let mut foreign = original.clone();
    foreign.associated_types[0].member = AssociatedTypeMemberId::migrate(SEED, 99);
    foreign
        .associated_types
        .sort_by_key(|binding| binding.member);
    replace_implementation(&mut f, foreign.clone());
    semantic(OwnedAssociatedAdmission::new().validate_bindings(&f.source, f.scheme, &foreign));
    let scalar = ty(&mut f.source, TypeForm::I64);
    let mut unused = original;
    unused
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == f.spare)
        .expect("unused binding")
        .value = AssociatedTypeBindingValue::Owned { ty: scalar };
    replace_implementation(&mut f, unused.clone());
    semantic(OwnedAssociatedAdmission::new().validate_bindings(&f.source, f.scheme, &unused));
}

#[test]
fn owned_associated_stored_templates_reject_hidden_projection_and_foreign_parameters() {
    let mut f = fixture();
    let form = projection(&f, f.lexical);
    let projected = ty(&mut f.source, form);
    let hidden = ty(
        &mut f.source,
        TypeForm::Applied {
            declaration: f.nominal,
            arguments: vec![projected],
        },
    );
    let mut value = implementation(&f, f.scheme);
    value
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == f.spare)
        .expect("unused binding")
        .value = AssociatedTypeBindingValue::Owned { ty: hidden };
    replace_implementation(&mut f, value.clone());
    semantic(OwnedAssociatedAdmission::new().validate_bindings(&f.source, f.scheme, &value));
    let foreign_parameter = ty(
        &mut f.source,
        TypeForm::TypeParameter {
            parameter: TypeParameterId::migrate(SEED, 2),
        },
    );
    value
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == f.spare)
        .expect("unused binding")
        .value = AssociatedTypeBindingValue::Owned {
        ty: foreign_parameter,
    };
    replace_implementation(&mut f, value.clone());
    semantic(OwnedAssociatedAdmission::new().validate_bindings(&f.source, f.scheme, &value));
    let field = FieldId::migrate(SEED, 0);
    if let Some(OwnerRecord::Field(field)) = f.source.owners.get_mut(&OwnerKey::Field(field)) {
        field.ty = projected;
    }
    let nominal = ty(
        &mut f.source,
        TypeForm::Applied {
            declaration: f.nominal,
            arguments: vec![f.cell],
        },
    );
    value
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == f.spare)
        .expect("unused binding")
        .value = AssociatedTypeBindingValue::Owned { ty: nominal };
    replace_implementation(&mut f, value.clone());
    semantic(OwnedAssociatedAdmission::new().validate_bindings(&f.source, f.scheme, &value));
}

#[test]
fn owned_associated_resolution_keeps_selected_identity_and_rebinds_lexical_scope() {
    let f = fixture();
    let admission = OwnedAssociatedAdmission::new();
    let operand = ImplementationOperand::Concrete {
        implementation: f.scheme,
        type_arguments: vec![f.buffer],
        implementations: vec![],
    };
    let result = admission
        .resolve_form(
            &f.source,
            &abstract_member(&f),
            AssociatedTypeContext::ContractSignature(f.contract),
            &operand,
            AssociatedTypeContext::Forbidden,
            GENERATION,
        )
        .expect("exact selected binding");
    let template = implementation(&f, f.scheme)
        .associated_types
        .into_iter()
        .find(|binding| binding.member == f.member)
        .expect("binding")
        .ty();
    assert_eq!(
        result,
        AssociatedResolution::Binding {
            implementation: f.scheme,
            template,
            bindings: BTreeMap::from([(f.scheme_parameter, f.buffer)])
        }
    );
    let rebound = ImplementationOperand::Parameter {
        scope: f.other_lexical,
        parameter: f.witness,
    };
    let result = admission
        .resolve_form(
            &f.source,
            &projection(&f, f.lexical),
            AssociatedTypeContext::Lexical(f.lexical),
            &rebound,
            AssociatedTypeContext::Lexical(f.other_lexical),
            GENERATION,
        )
        .expect("capture-free caller rebinding");
    assert_eq!(
        result,
        AssociatedResolution::Lexical(projection(&f, f.other_lexical))
    );
    let concrete = ImplementationOperand::Concrete {
        implementation: f.concrete,
        type_arguments: vec![],
        implementations: vec![],
    };
    assert_eq!(
        admission
            .resolve_form(
                &f.source,
                &abstract_member(&f),
                AssociatedTypeContext::ContractSignature(f.contract),
                &concrete,
                AssociatedTypeContext::Forbidden,
                GENERATION
            )
            .expect("zero ordinary actuals still select a member"),
        AssociatedResolution::Binding {
            implementation: f.concrete,
            template: f.buffer,
            bindings: BTreeMap::new()
        }
    );
}

#[test]
fn owned_associated_binding_work_and_storage_exact_fit_and_one_less() {
    let f = fixture();
    let value = implementation(&f, f.scheme);
    let measured = OwnedAssociatedAdmission::new();
    measured
        .validate_bindings(&f.source, f.scheme, &value)
        .expect("measure complete binding admission");
    let (steps, bytes) = measured.work();
    assert!(steps > 1 && bytes > 1);
    let limits = AssociatedAdmissionLimits {
        maximum_steps: Some(steps),
        maximum_metadata_bytes: Some(bytes),
        maximum_depth: 256,
    };
    OwnedAssociatedAdmission::with_limits(limits)
        .validate_bindings(&f.source, f.scheme, &value)
        .expect("exact admission fit");
    for limits in [
        AssociatedAdmissionLimits {
            maximum_steps: Some(steps - 1),
            ..limits
        },
        AssociatedAdmissionLimits {
            maximum_metadata_bytes: Some(bytes - 1),
            ..limits
        },
    ] {
        assert_eq!(
            OwnedAssociatedAdmission::with_limits(limits)
                .validate_bindings(&f.source, f.scheme, &value)
                .expect_err("one less capacity refuses")
                .class,
            DiagnosticClass::Resource
        );
    }
    let shared = OwnedAssociatedAdmission::with_limits(limits);
    shared
        .validate_bindings(&f.source, f.scheme, &value)
        .expect("first complete admission fits");
    assert_eq!(
        shared
            .validate_bindings(&f.source, f.scheme, &value)
            .expect_err("temporary readers cannot reset aggregate work")
            .class,
        DiagnosticClass::Resource
    );
}

struct ControlledRead<'a> {
    source: &'a KernelSnapshot,
    control: crate::platform::execution::ExecutionControl,
    remaining: Cell<usize>,
}
impl ExpressionRead for ControlledRead<'_> {
    fn package_id(&self) -> PackageId {
        self.source.package_id()
    }
    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.source.owner(owner)
    }
    fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        self.source.type_object(ty)
    }
    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.source.package_interface_owner(package, owner)
    }
    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.source.has_dependency(package)
    }
    fn validation_work(&self) -> Result<(), Diagnostic> {
        if self.remaining.get() == 0 {
            self.control.cancel();
        } else {
            self.remaining.set(self.remaining.get() - 1);
        }
        self.control
            .check()
            .map_err(crate::platform::execution::normalized::execution_diagnostic)
    }
}

#[test]
fn owned_associated_real_control_cancels_inventory_and_healthy_retry() {
    let f = fixture();
    let control = ControlledRead {
        source: &f.source,
        control: crate::platform::execution::ExecutionControl::uncancelled(),
        remaining: Cell::new(4),
    };
    let result = OwnedAssociatedAdmission::new().validate_bindings(
        &control,
        f.scheme,
        &implementation(&f, f.scheme),
    );
    assert_eq!(
        result
            .expect_err("actual owning control must interrupt cached metadata")
            .class,
        DiagnosticClass::Cancelled
    );
    assert!(control.control.is_cancelled());
    OwnedAssociatedAdmission::new()
        .validate_bindings(&f.source, f.scheme, &implementation(&f, f.scheme))
        .expect("independent healthy retry");
}

#[test]
fn owned_associated_shared_dag_revisits_greater_occurrence_depth() {
    let mut f = fixture();
    let shared = ty(&mut f.source, TypeForm::OwnedSequence { item: f.cell });
    let deep = ty(&mut f.source, TypeForm::OwnedSequence { item: shared });
    // LIFO traversal sees the shared node shallow first. Its deeper occurrence
    // must still revisit the cell below it, which is three edges from this root.
    let root = ty(
        &mut f.source,
        TypeForm::OwnedProduct {
            fields: vec![
                StructuralTypeField {
                    name: name("deep"),
                    ty: deep,
                },
                StructuralTypeField {
                    name: name("shallow"),
                    ty: shared,
                },
            ],
        },
    );
    let limits = AssociatedAdmissionLimits {
        maximum_depth: 3,
        ..AssociatedAdmissionLimits::default()
    };
    OwnedAssociatedAdmission::with_limits(limits)
        .validate_occurrences(
            &f.source,
            &[root],
            AssociatedTypeContext::Forbidden,
            GENERATION,
        )
        .expect("complete shared DAG fits three edges");
    assert_eq!(
        OwnedAssociatedAdmission::with_limits(AssociatedAdmissionLimits {
            maximum_depth: 2,
            ..limits
        })
        .validate_occurrences(
            &f.source,
            &[root],
            AssociatedTypeContext::Forbidden,
            GENERATION
        )
        .expect_err("shallow visitation cannot hide deeper descendants")
        .class,
        DiagnosticClass::Resource
    );
}

struct RepresentationRead<'a> {
    source: &'a KernelSnapshot,
    imports: BTreeMap<(PackageId, OwnerKey), PackageInterfaceRecord>,
    representations: BTreeMap<(PackageId, OwnerKey), PackageInterfaceRecord>,
}

impl ExpressionRead for RepresentationRead<'_> {
    fn package_id(&self) -> PackageId {
        self.source.package_id()
    }
    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.source.owner(owner)
    }
    fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        self.source.type_object(ty)
    }
    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        Ok(self.imports.get(&(package, owner)).cloned())
    }
    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        Ok(self
            .imports
            .keys()
            .any(|(candidate, _)| *candidate == package))
    }
    fn package_representation_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        Ok(self
            .imports
            .get(&(package, owner))
            .or_else(|| self.representations.get(&(package, owner)))
            .cloned())
    }
    fn has_representation_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        Ok(self.has_dependency(package)?
            || self
                .representations
                .keys()
                .any(|(candidate, _)| *candidate == package))
    }
}

#[test]
fn owned_associated_private_generic_layout_property_does_not_require_public_authority() {
    let mut f = fixture();
    let imported = PackageId::migrate(SEED, 1);
    let hidden = PackageId::migrate(SEED, 2);
    let public_id = DeclarationId::migrate(SEED, 30);
    let private_id = DeclarationId::migrate(SEED, 31);
    let public = DeclarationReference {
        package: imported,
        declaration: public_id,
    };
    let private = DeclarationReference {
        package: hidden,
        declaration: private_id,
    };
    let formal = TypeParameterId::migrate(SEED, 30);
    let variable = ty(&mut f.source, TypeForm::TypeParameter { parameter: formal });
    let private_type = ty(
        &mut f.source,
        TypeForm::Applied {
            declaration: private,
            arguments: vec![f.cell],
        },
    );
    let public_type = ty(
        &mut f.source,
        TypeForm::Named {
            declaration: public,
        },
    );
    let mut value = implementation(&f, f.scheme);
    value
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == f.spare)
        .expect("unused binding")
        .value = AssociatedTypeBindingValue::Owned { ty: public_type };
    replace_implementation(&mut f, value.clone());
    let mut read = RepresentationRead {
        source: &f.source,
        imports: BTreeMap::new(),
        representations: BTreeMap::new(),
    };
    for (package, id, ordinal, field_type, parameters) in [
        (imported, public_id, 30, private_type, vec![]),
        (hidden, private_id, 31, variable, vec![formal]),
    ] {
        let records = if package == imported {
            &mut read.imports
        } else {
            &mut read.representations
        };
        let field = FieldId::migrate(SEED, ordinal);
        let declaration = PackageInterfaceDeclaration {
            header: OwnerHeader::new(OwnerKey::Declaration(id), OwnerKind::OwnedRecord),
            name: name("OpaqueRecord"),
            payload: PackageInterfaceDeclarationPayload::OwnedRecord {
                type_parameters: parameters,
                fields: vec![field],
            },
        };
        records.insert(
            (package, OwnerKey::Declaration(id)),
            if package == imported {
                PackageInterfaceRecord::Declaration(declaration)
            } else {
                PackageInterfaceRecord::RepresentationDeclaration(declaration)
            },
        );
        records.insert(
            (package, OwnerKey::Field(field)),
            PackageInterfaceRecord::Field(FieldRecord {
                header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
                declaration: id,
                name: name("value"),
                ty: field_type,
            }),
        );
    }
    read.representations.insert(
        (hidden, OwnerKey::TypeParameter(formal)),
        PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(formal), OwnerKind::TypeParameter),
            declaration: private_id,
            name: name("Hidden"),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    assert!(!read.has_dependency(hidden).expect("direct import lookup"));
    assert!(
        read.package_interface_owner(hidden, OwnerKey::Declaration(private_id))
            .expect("public point read")
            .is_none()
    );
    OwnedAssociatedAdmission::new().validate_bindings(&read, f.scheme, &value)
        .expect("complete projection absence and Owned proof follow private generic representation metadata");
    let hidden_projection = TypeForm::AssociatedWitnessProjection {
        scope: private,
        parameter: f.witness,
        contract: f.contract,
        member: f.member,
    };
    semantic(OwnedAssociatedAdmission::new().validate_form(
        &read,
        &hidden_projection,
        AssociatedTypeContext::Lexical(private),
        GENERATION,
    ));
}

fn environment<'a>(
    ordinary: &'a BTreeMap<TypeParameterId, TypeObjectDigest>,
    source: AssociatedTypeContext,
    caller: AssociatedTypeContext,
    contract_operand: Option<&'a ImplementationOperand>,
    implementations: &'a BTreeMap<ImplementationParameterId, ImplementationOperand>,
) -> parallel_types::AssociatedTypeEnvironment<'a> {
    parallel_types::AssociatedTypeEnvironment {
        ordinary,
        source,
        caller,
        consumer_generation: GENERATION,
        contract_selection: contract_operand
            .map(parallel_types::AssociatedContractSelection::Witness),
        implementations,

        scopes: &EMPTY_READ_SCOPES,
    }
}

#[test]
fn owned_associated_engine_empty_ordinary_map_still_selects_concrete_member() {
    let mut f = fixture();
    let form = abstract_member(&f);
    let root = ty(&mut f.source, form);
    let ordinary = BTreeMap::new();
    let implementations = BTreeMap::new();
    let operand = ImplementationOperand::Concrete {
        implementation: f.concrete,
        type_arguments: vec![],
        implementations: vec![],
    };
    let mut derived = parallel_types::WitnessAppliedTypes::new(
        &f.source,
        environment(
            &ordinary,
            AssociatedTypeContext::ContractSignature(f.contract),
            AssociatedTypeContext::Forbidden,
            Some(&operand),
            &implementations,
        ),
    );
    assert_eq!(
        derived
            .substitute(root, 0)
            .expect("zero ordinary width still selects a binding"),
        f.buffer
    );
}

#[test]
fn owned_associated_engine_immutable_witness_environments_have_distinct_results() {
    let mut f = fixture();
    let form = abstract_member(&f);
    let root = ty(&mut f.source, form);
    let ordinary = BTreeMap::new();
    let implementations = BTreeMap::new();
    let mut results = Vec::new();
    for actual in [f.cell, f.buffer] {
        let operand = ImplementationOperand::Concrete {
            implementation: f.scheme,
            type_arguments: vec![actual],
            implementations: vec![],
        };
        let mut derived = parallel_types::WitnessAppliedTypes::new(
            &f.source,
            environment(
                &ordinary,
                AssociatedTypeContext::ContractSignature(f.contract),
                AssociatedTypeContext::Forbidden,
                Some(&operand),
                &implementations,
            ),
        );
        results.push(
            derived
                .substitute(root, 0)
                .expect("exact immutable selected application"),
        );
    }
    assert_eq!(results, vec![f.cell, f.buffer]);
}

#[test]
fn owned_associated_engine_lexical_rebinding_retains_caller_object() {
    let mut f = fixture();
    let form = projection(&f, f.lexical);
    let root = ty(&mut f.source, form);
    let ordinary = BTreeMap::new();
    let implementations = BTreeMap::from([(
        f.witness,
        ImplementationOperand::Parameter {
            scope: f.other_lexical,
            parameter: f.witness,
        },
    )]);
    let mut derived = parallel_types::WitnessAppliedTypes::new(
        &f.source,
        environment(
            &ordinary,
            AssociatedTypeContext::Lexical(f.lexical),
            AssociatedTypeContext::Lexical(f.other_lexical),
            None,
            &implementations,
        ),
    );
    let result = derived
        .substitute(root, 0)
        .expect("caller lexical identity is rebound");
    let expected =
        TypeObject::new(projection(&f, f.other_lexical)).expect("caller projection type");
    assert_ne!(root, result);
    assert_eq!(
        result,
        encode_type_object(&expected)
            .expect("caller canonical type")
            .0
    );
    assert_eq!(derived.into_types().get(&result), Some(&expected));
}

#[test]
fn owned_associated_engine_simultaneous_actual_projection_is_not_recaptured() {
    let mut f = fixture();
    let form = projection(&f, f.other_lexical);
    let caller_projection = ty(&mut f.source, form);
    let callee_parameter = ty(
        &mut f.source,
        TypeForm::TypeParameter {
            parameter: f.scheme_parameter,
        },
    );
    let root = ty(
        &mut f.source,
        TypeForm::OwnedSequence {
            item: callee_parameter,
        },
    );
    let ordinary = BTreeMap::from([(f.scheme_parameter, caller_projection)]);
    // No witness mapping exists in the source environment. Visiting an inserted
    // caller actual under that environment would reject or capture it.
    let implementations = BTreeMap::new();
    let mut derived = parallel_types::WitnessAppliedTypes::new(
        &f.source,
        environment(
            &ordinary,
            AssociatedTypeContext::Lexical(f.scheme),
            AssociatedTypeContext::Lexical(f.other_lexical),
            None,
            &implementations,
        ),
    );
    let result = derived
        .substitute(root, 0)
        .expect("caller actuals are opaque simultaneous replacements");
    let expected = TypeObject::new(TypeForm::OwnedSequence {
        item: caller_projection,
    })
    .expect("caller sequence type");
    assert_eq!(
        result,
        encode_type_object(&expected)
            .expect("canonical caller sequence")
            .0
    );
    assert_eq!(derived.into_types().get(&result), Some(&expected));
}

#[test]
fn owned_associated_engine_metadata_exact_fit_and_refused_retry_never_memoizes() {
    let mut f = fixture();
    let form = projection(&f, f.lexical);
    let projection_root = ty(&mut f.source, form);
    let root = ty(
        &mut f.source,
        TypeForm::OwnedSequence {
            item: projection_root,
        },
    );
    let ordinary = BTreeMap::new();
    let implementations = BTreeMap::from([(
        f.witness,
        ImplementationOperand::Parameter {
            scope: f.other_lexical,
            parameter: f.witness,
        },
    )]);
    let make_environment = || {
        environment(
            &ordinary,
            AssociatedTypeContext::Lexical(f.lexical),
            AssociatedTypeContext::Lexical(f.other_lexical),
            None,
            &implementations,
        )
    };
    let mut measured = parallel_types::WitnessAppliedTypes::new(&f.source, make_environment());
    let expected = measured
        .substitute(root, 0)
        .expect("measure complete associated derivation");
    let bytes = measured.metadata_bytes_for_test();
    assert!(bytes > 1);
    let mut exact = parallel_types::WitnessAppliedTypes::with_metadata_limit(
        &f.source,
        make_environment(),
        bytes,
    );
    assert_eq!(
        exact
            .substitute(root, 0)
            .expect("exact derivation storage fit"),
        expected
    );
    assert_eq!(exact.metadata_bytes_for_test(), bytes);
    assert_eq!(
        exact
            .substitute(root, 0)
            .expect("same immutable environment memo hit"),
        expected
    );
    assert_eq!(exact.metadata_bytes_for_test(), bytes);
    assert!(exact.into_types().contains_key(&expected));
    let mut refused = parallel_types::WitnessAppliedTypes::with_metadata_limit(
        &f.source,
        make_environment(),
        bytes - 1,
    );
    for _ in 0..2 {
        assert_eq!(
            refused
                .substitute(root, 0)
                .expect_err("refused derivation cannot return cached output")
                .class,
            DiagnosticClass::Resource
        );
    }
    assert!(!refused.into_types().contains_key(&expected));
}

#[test]
fn owned_associated_binding_accepts_unrestricted_ordinary_nominal_formals() {
    let mut f = fixture();
    let module = match f
        .source
        .owners
        .get(&OwnerKey::Declaration(f.nominal.declaration))
        .expect("nominal declaration")
    {
        OwnerRecord::Declaration(record) => record.module,
        _ => panic!("nominal declaration kind"),
    };
    let formal = TypeParameterId::migrate(SEED, 50);
    let field = FieldId::migrate(SEED, 50);
    let generic = declaration(
        &mut f.source,
        module,
        50,
        "Ordinary",
        OwnerKind::Record,
        DeclarationPayload::Record {
            type_parameters: vec![formal],
            fields: vec![field],
        },
    );
    let variable = parameter(&mut f.source, generic, formal);
    let Some(OwnerRecord::TypeParameter(record)) =
        f.source.owners.get_mut(&OwnerKey::TypeParameter(formal))
    else {
        panic!("ordinary formal")
    };
    record.constraints = TypeParameterConstraints::None;
    f.source.owners.insert(
        OwnerKey::Field(field),
        OwnerRecord::Field(FieldRecord {
            header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
            declaration: generic.declaration,
            name: name("data"),
            ty: variable,
        }),
    );
    let scalar = ty(&mut f.source, TypeForm::I64);
    let data = ty(
        &mut f.source,
        TypeForm::Applied {
            declaration: generic,
            arguments: vec![scalar],
        },
    );
    let Some(OwnerRecord::Field(field)) = f
        .source
        .owners
        .get_mut(&OwnerKey::Field(FieldId::migrate(SEED, 0)))
    else {
        panic!("owned record field")
    };
    field.ty = data;
    let nominal = ty(
        &mut f.source,
        TypeForm::Applied {
            declaration: f.nominal,
            arguments: vec![f.cell],
        },
    );
    let mut value = implementation(&f, f.scheme);
    value
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == f.spare)
        .expect("unused binding")
        .value = AssociatedTypeBindingValue::Owned { ty: nominal };
    replace_implementation(&mut f, value.clone());
    OwnedAssociatedAdmission::new()
        .validate_bindings(&f.source, f.scheme, &value)
        .expect("raw ordinary nominal formals have their own category proof");
    let form = projection(&f, f.lexical);
    let projected = ty(&mut f.source, form);
    let Some(OwnerRecord::Field(field)) = f
        .source
        .owners
        .get_mut(&OwnerKey::Field(FieldId::migrate(SEED, 50)))
    else {
        panic!("ordinary field")
    };
    field.ty = projected;
    semantic(OwnedAssociatedAdmission::new().validate_bindings(&f.source, f.scheme, &value));
}

#[test]
fn owned_associated_own_definition_binding_is_opaque_to_contract_ordinary_map() {
    let mut f = fixture();
    let form = abstract_member(&f);
    let root = ty(&mut f.source, form);
    let value = implementation(&f, f.scheme);
    let expected = value
        .associated_types
        .iter()
        .find(|binding| binding.member == f.member)
        .expect("binding")
        .ty();
    // An unrelated source substitution with the same key cannot reinterpret an
    // implementation-context RHS. The engine environment is not an application
    // certificate; this adversary deliberately makes capture easy to observe.
    let ordinary = BTreeMap::from([(f.scheme_parameter, f.buffer)]);
    let implementations = BTreeMap::new();
    let associated = OwnedAssociatedAdmission::new();
    let mut derived = parallel_types::WitnessAppliedTypes::with_admission(
        &f.source,
        parallel_types::AssociatedTypeEnvironment {
            ordinary: &ordinary,
            source: AssociatedTypeContext::ContractSignature(f.contract),
            caller: AssociatedTypeContext::Lexical(f.scheme),
            consumer_generation: GENERATION,
            contract_selection: Some(parallel_types::AssociatedContractSelection::Definition {
                scope: f.scheme,
                implementation: &value,
            }),
            implementations: &implementations,

            scopes: &EMPTY_READ_SCOPES,
        },
        &associated,
    );
    assert_eq!(
        derived
            .substitute(root, 0)
            .expect("own-definition template retains its lexical formals"),
        expected
    );
    assert_ne!(expected, f.buffer);
}

#[test]
fn owned_associated_representation_evidence_cannot_name_contract_or_witness_authority() {
    let mut f = fixture();
    let hidden = PackageId::migrate(SEED, 60);
    let foreign = DeclarationReference {
        package: hidden,
        declaration: DeclarationId::migrate(SEED, 60),
    };
    let value = owned_contract::contract_record(&f.source, f.contract).expect("source contract");
    let foreign_record = PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
        header: OwnerHeader::new(
            OwnerKey::Declaration(foreign.declaration),
            OwnerKind::OwnedContract,
        ),
        name: name("HiddenContract"),
        payload: PackageInterfaceDeclarationPayload::OwnedContract(value),
    });
    let Some(OwnerRecord::Declaration(record)) = f
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(f.lexical.declaration))
    else {
        panic!("lexical function")
    };
    let DeclarationPayload::Function(value) = &mut record.payload else {
        panic!("function payload")
    };
    value.implementation_parameters[0].contract = foreign;
    let mut read = RepresentationRead {
        source: &f.source,
        imports: BTreeMap::new(),
        representations: BTreeMap::from([(
            (hidden, OwnerKey::Declaration(foreign.declaration)),
            foreign_record.clone(),
        )]),
    };
    let member = TypeForm::AssociatedContractMember {
        contract: foreign,
        member: f.member,
    };
    let lexical = TypeForm::AssociatedWitnessProjection {
        scope: f.lexical,
        parameter: f.witness,
        contract: foreign,
        member: f.member,
    };
    semantic(OwnedAssociatedAdmission::new().validate_form(
        &read,
        &member,
        AssociatedTypeContext::ContractSignature(foreign),
        GENERATION,
    ));
    semantic(OwnedAssociatedAdmission::new().validate_form(
        &read,
        &lexical,
        AssociatedTypeContext::Lexical(f.lexical),
        GENERATION,
    ));
    read.imports.insert(
        (hidden, OwnerKey::Declaration(foreign.declaration)),
        foreign_record,
    );
    OwnedAssociatedAdmission::new()
        .validate_form(
            &read,
            &member,
            AssociatedTypeContext::ContractSignature(foreign),
            GENERATION,
        )
        .expect("direct authored contract authority is distinct from representation evidence");
    OwnedAssociatedAdmission::new()
        .validate_form(
            &read,
            &lexical,
            AssociatedTypeContext::Lexical(f.lexical),
            GENERATION,
        )
        .expect("exact lexical witness can name a declared direct contract");
}

#[test]
fn owned_associated_function_carrier_retains_exact_witness_environment_and_source_generations() {
    let mut f = fixture();
    let form = projection(&f, f.lexical);
    let root = ty(&mut f.source, form);
    let Some(OwnerRecord::Declaration(record)) = f
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(f.lexical.declaration))
    else {
        panic!("callee function")
    };
    let DeclarationPayload::Function(value) = &mut record.payload else {
        panic!("callee payload")
    };
    value.result.ty = root;
    let operands = [ImplementationOperand::Concrete {
        implementation: f.scheme,
        type_arguments: vec![f.buffer],
        implementations: vec![],
    }];
    let occurrence = OwnedTypeOccurrence::authored(Some(f.other_lexical), GENERATION);
    let associated = OwnedAssociatedAdmission::new();
    let application = owned_contract::admit_function_application(
        &f.source,
        f.lexical,
        &[f.cell],
        &operands,
        occurrence,
        &associated,
    )
    .expect("complete selected function application");
    assert_eq!(application.signature.result.ty, root);
    {
        let overlay = owned_contract::AppliedTypeRead {
            read: &f.source,
            types: &application.types,
        };
        let mut derived = parallel_types::WitnessAppliedTypes::with_admission(
            &overlay,
            application.environment(),
            &associated,
        );
        assert_eq!(
            derived
                .substitute(root, 0)
                .expect("carrier preserves witness selection"),
            f.buffer
        );
    }
    let Some(OwnerRecord::Declaration(record)) = f
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(f.lexical.declaration))
    else {
        panic!("callee function")
    };
    record.header.contract_version = 32;
    assert_eq!(
        owned_contract::admit_function_application(
            &f.source,
            f.lexical,
            &[f.cell],
            &operands,
            occurrence,
            &OwnedAssociatedAdmission::new()
        )
        .err()
        .expect("old source result owner must refuse before a concrete selection erases projection")
        .class,
        DiagnosticClass::Semantic
    );
}

#[test]
fn owned_associated_full_application_budget_is_cumulative_across_calls() {
    let f = fixture();
    let operands = [ImplementationOperand::Concrete {
        implementation: f.scheme,
        type_arguments: vec![f.buffer],
        implementations: vec![],
    }];
    let occurrence = OwnedTypeOccurrence::authored(Some(f.other_lexical), GENERATION);
    let measured = OwnedAssociatedAdmission::new();
    owned_contract::admit_function_application(
        &f.source,
        f.lexical,
        &[f.cell],
        &operands,
        occurrence,
        &measured,
    )
    .expect("measure complete application");
    let (steps, bytes) = measured.work();
    let limits = AssociatedAdmissionLimits {
        maximum_steps: Some(steps),
        maximum_metadata_bytes: Some(bytes),
        maximum_depth: 256,
    };
    let associated = OwnedAssociatedAdmission::with_limits(limits);
    owned_contract::admit_function_application(
        &f.source,
        f.lexical,
        &[f.cell],
        &operands,
        occurrence,
        &associated,
    )
    .expect("exact complete application fit");
    assert_eq!(
        owned_contract::admit_function_application(
            &f.source,
            f.lexical,
            &[f.cell],
            &operands,
            occurrence,
            &associated
        )
        .err()
        .expect("a second application cannot reset shared proof or storage budget")
        .class,
        DiagnosticClass::Resource
    );
}

#[test]
fn owned_associated_occurrence_rejects_mixed_authority_scopes() {
    let f = fixture();
    OwnedTypeOccurrence::authored(Some(f.lexical), GENERATION)
        .validate_context()
        .expect("matching authored scope");
    OwnedTypeOccurrence::contract(f.contract, GENERATION)
        .validate_context()
        .expect("matching contract signature scope");
    OwnedTypeOccurrence::representation(f.nominal, GENERATION)
        .validate_context()
        .expect("representation authority has no associated scope");
    for parameters in [
        TypeParameterContext::Closed,
        TypeParameterContext::Authored(f.other_lexical),
        TypeParameterContext::NominalRepresentation(f.nominal),
    ] {
        semantic(
            OwnedTypeOccurrence {
                parameters,
                associated: AssociatedTypeContext::Lexical(f.lexical),
                consumer_generation: GENERATION,
            }
            .validate_context(),
        );
    }
}

#[test]
fn owned_associated_function_parameter_requires_its_own_generation() {
    let mut f = fixture();
    let form = projection(&f, f.lexical);
    let projected = ty(&mut f.source, form);
    let id = ParameterId::migrate(SEED, 60);
    let Some(OwnerRecord::Declaration(record)) = f
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(f.lexical.declaration))
    else {
        panic!("callee source declaration")
    };
    let DeclarationPayload::Function(value) = &mut record.payload else {
        panic!("callee source function")
    };
    value.parameters = vec![id];
    f.source.owners.insert(
        OwnerKey::Parameter(id),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
            parent: ParameterParent::Function(f.lexical.declaration),
            name: name("projected"),
            ty: projected,
            use_mode: ParameterUse::Consume,
            resource_requirement: None,
        }),
    );
    let operands = [ImplementationOperand::Concrete {
        implementation: f.scheme,
        type_arguments: vec![f.buffer],
        implementations: vec![],
    }];
    let occurrence = OwnedTypeOccurrence::authored(Some(f.other_lexical), GENERATION);
    let application = owned_contract::admit_function_application(
        &f.source,
        f.lexical,
        &[f.cell],
        &operands,
        occurrence,
        &OwnedAssociatedAdmission::new(),
    )
    .expect("current parent and parameter independently consume the projected template");
    assert_eq!(application.parameters[0].ty, projected);
    let Some(OwnerRecord::Parameter(record)) = f.source.owners.get_mut(&OwnerKey::Parameter(id))
    else {
        panic!("callee source parameter")
    };
    record.header.contract_version = 32;
    assert_eq!(
        owned_contract::admit_function_application(
            &f.source,
            f.lexical,
            &[f.cell],
            &operands,
            occurrence,
            &OwnedAssociatedAdmission::new()
        )
        .err()
        .expect("a current parent cannot authorize an old parameter projection")
        .class,
        DiagnosticClass::Semantic
    );
}

#[test]
fn owned_associated_function_actual_projection_is_owned_without_extra_properties() {
    let mut f = fixture();
    let module = match f
        .source
        .owners
        .get(&OwnerKey::Declaration(f.lexical.declaration))
        .expect("lexical source")
    {
        OwnerRecord::Declaration(record) => record.module,
        _ => panic!("lexical declaration kind"),
    };
    let formal = TypeParameterId::migrate(SEED, 61);
    let callee = DeclarationReference {
        package: f.source.root.package_id,
        declaration: DeclarationId::migrate(SEED, 61),
    };
    let raw_result = parameter(&mut f.source, callee, formal);
    declaration(
        &mut f.source,
        module,
        61,
        "ProjectedActual",
        OwnerKind::PureFunction,
        DeclarationPayload::Function(function(formal, raw_result, vec![])),
    );
    let form = projection(&f, f.other_lexical);
    let actual = ty(&mut f.source, form);
    let occurrence = OwnedTypeOccurrence::authored(Some(f.other_lexical), GENERATION);
    let associated = OwnedAssociatedAdmission::new();
    let application = owned_contract::admit_function_application(
        &f.source,
        callee,
        &[actual],
        &[],
        occurrence,
        &associated,
    )
    .expect("an exact caller projection satisfies an Owned-only ordinary formal");
    {
        let overlay = owned_contract::AppliedTypeRead {
            read: &f.source,
            types: &application.types,
        };
        let mut derived = parallel_types::WitnessAppliedTypes::with_admission(
            &overlay,
            application.environment(),
            &associated,
        );
        assert_eq!(
            derived
                .substitute(raw_result, 0)
                .expect("inserted caller projection stays opaque"),
            actual
        );
    }
    for constraints in [
        TypeParameterConstraints::None,
        TypeParameterConstraints::OwnedTransferable,
        TypeParameterConstraints::OwnedShareable,
    ] {
        let Some(OwnerRecord::TypeParameter(record)) =
            f.source.owners.get_mut(&OwnerKey::TypeParameter(formal))
        else {
            panic!("callee formal")
        };
        record.constraints = constraints;
        assert_eq!(
            owned_contract::admit_function_application(
                &f.source,
                callee,
                &[actual],
                &[],
                occurrence,
                &OwnedAssociatedAdmission::new()
            )
            .err()
            .expect("Owned alone grants neither ordinary category nor Transfer/Share")
            .class,
            DiagnosticClass::Semantic
        );
    }
    let Some(OwnerRecord::TypeParameter(record)) =
        f.source.owners.get_mut(&OwnerKey::TypeParameter(formal))
    else {
        panic!("callee formal")
    };
    record.constraints = TypeParameterConstraints::Owned;
    assert_eq!(
        owned_contract::admit_function_application(
            &f.source,
            callee,
            &[actual],
            &[],
            OwnedTypeOccurrence {
                consumer_generation: 32,
                ..occurrence
            },
            &OwnedAssociatedAdmission::new()
        )
        .err()
        .expect("old consuming application owner cannot use a projected phantom actual")
        .class,
        DiagnosticClass::Semantic
    );
}

#[test]
fn owned_associated_many_scalar_occurrences_fit_without_reusing_authority() {
    let mut f = fixture();
    let scalar = ty(&mut f.source, TypeForm::I64);
    let admission = OwnedAssociatedAdmission::new();
    // Complete compiler graphs repeatedly classify ordinary scalar parameters,
    // locals and call results under one cumulative operation budget.
    for _ in 0..100_000 {
        admission
            .validate_occurrences(
                &f.source,
                &[scalar],
                AssociatedTypeContext::Lexical(f.lexical),
                GENERATION,
            )
            .expect("scalar classifications fit the unchanged default budget");
    }
    let (steps, bytes) = admission.work();
    assert!(steps >= 100_000 && bytes > 0);
    assert!(bytes < 64 * 1_048_576, "fixture metadata regression bound");
    assert!(!admission.metadata_observation().saturated);

    // The same digest must still reread canonical content on the next call.
    f.source.types.get_mut(&scalar).unwrap().contract_version = 0;
    assert_eq!(
        admission
            .validate_occurrences(
                &f.source,
                &[scalar],
                AssociatedTypeContext::Lexical(f.lexical),
                GENERATION,
            )
            .expect_err("changed authority is checked again")
            .code,
        "kernel_type_contract"
    );
}

#[test]
fn owned_associated_repeated_deep_walks_reuse_queue_storage_with_exact_contexts() {
    let mut f = fixture();
    let mut root = f.cell;
    for _ in 0..contract::MAXIMUM_TYPE_DEPTH - 1 {
        root = ty(
            &mut f.source,
            TypeForm::OwnedProduct {
                fields: vec![StructuralTypeField {
                    name: name("owner"),
                    ty: root,
                }],
            },
        );
    }
    let associated = OwnedAssociatedAdmission::new();
    // A shared owning operation may classify many overlapping deep signatures.
    // Each call rereads its authority; compact per-walk coordinates and actual
    // queue capacity keep the unchanged cumulative storage bound meaningful.
    for _ in 0..1_000 {
        associated
            .validate_occurrences(
                &f.source,
                &[root],
                AssociatedTypeContext::Lexical(f.lexical),
                GENERATION,
            )
            .expect("valid repeated near-boundary traversal fits the existing budget");
    }
    let (steps, bytes) = associated.work();
    assert!(
        steps > 1_000 && bytes < 64 * 1_048_576,
        "fixture metadata regression bound"
    );
    assert!(!associated.metadata_observation().saturated);
    // A different read is not a cache hit even when semantic coordinates match.
    let form = projection(&f, f.lexical);
    let projected = ty(&mut f.source, form);
    let Some(OwnerRecord::Declaration(record)) = f
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(f.lexical.declaration))
    else {
        panic!("lexical source")
    };
    record.header.contract_version = 32;
    semantic(associated.validate_occurrences(
        &f.source,
        &[projected],
        AssociatedTypeContext::Lexical(f.lexical),
        GENERATION,
    ));
}

#[test]
fn owned_associated_nominal_absence_keeps_scope_independent_work_within_shared_storage() {
    let mut f = fixture();
    let module = ModuleId::migrate(SEED, 20000);
    let unit = ty(&mut f.source, TypeForm::Unit);
    let mut fields = Vec::new();
    let mut last = None;
    for position in 0..256 {
        let field = FieldId::migrate(SEED, 20000 + position);
        let nominal = declaration(
            &mut f.source,
            module,
            20000 + position,
            &format!("Target{position}"),
            OwnerKind::Record,
            DeclarationPayload::Record {
                type_parameters: vec![],
                fields: vec![field],
            },
        );
        f.source.owners.insert(
            OwnerKey::Field(field),
            OwnerRecord::Field(FieldRecord {
                header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
                declaration: nominal.declaration,
                name: name("value"),
                ty: unit,
            }),
        );
        let named = ty(
            &mut f.source,
            TypeForm::Named {
                declaration: nominal,
            },
        );
        fields.push(StructuralTypeField {
            name: name(&format!("target_{position:04}")),
            ty: named,
        });
        last = Some((field, nominal));
    }
    let root = ty(&mut f.source, TypeForm::StructuralRecord { fields });
    let limits = AssociatedAdmissionLimits {
        maximum_metadata_bytes: Some(128 * 1024),
        ..AssociatedAdmissionLimits::default()
    };
    let admission = OwnedAssociatedAdmission::with_limits(limits);
    admission
        .validate_occurrences(
            &f.source,
            &[root],
            AssociatedTypeContext::Forbidden,
            GENERATION,
        )
        .expect("all256 exact nominal fields fit without unused per-declaration absence scopes");
    assert!(
        admission.work().1
            < limits
                .maximum_metadata_bytes
                .expect("explicit test allowance")
    );

    // A shallower visit to Unit never suppresses a later depth refusal. Depth is
    // checked before the shared absence key may skip a completed ordinary node.
    let shallow = OwnedAssociatedAdmission::with_limits(AssociatedAdmissionLimits {
        maximum_depth: 1,
        ..limits
    });
    let depth = shallow
        .validate_occurrences(
            &f.source,
            &[root, unit],
            AssociatedTypeContext::Forbidden,
            GENERATION,
        )
        .unwrap_err();
    assert_eq!(depth.class, DiagnosticClass::Resource);
    assert_eq!(
        depth.message,
        "associated type traversal exceeds its depth admission"
    );

    let (field, nominal) = last.unwrap();
    let OwnerRecord::Field(record) = f.source.owners.get_mut(&OwnerKey::Field(field)).unwrap()
    else {
        panic!("last field")
    };
    record.declaration = f.nominal.declaration;
    assert_ne!(record.declaration, nominal.declaration);
    let foreign = OwnedAssociatedAdmission::with_limits(limits)
        .validate_occurrences(
            &f.source,
            &[root],
            AssociatedTypeContext::Forbidden,
            GENERATION,
        )
        .unwrap_err();
    assert_eq!(foreign.class, DiagnosticClass::Semantic);
}

#[test]
fn owned_associated_projection_free_nominal_fields_keep_exact_formal_authority() {
    let mut f = fixture();
    let module = ModuleId::migrate(SEED, 21000);
    let formal = TypeParameterId::migrate(SEED, 21000);
    let field = FieldId::migrate(SEED, 21000);
    let nominal = declaration(
        &mut f.source,
        module,
        21000,
        "Template",
        OwnerKind::OwnedRecord,
        DeclarationPayload::OwnedRecord {
            type_parameters: vec![formal],
            fields: vec![field],
        },
    );
    let formal_type = parameter(&mut f.source, nominal, formal);
    f.source.owners.insert(
        OwnerKey::Field(field),
        OwnerRecord::Field(FieldRecord {
            header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
            declaration: nominal.declaration,
            name: name("value"),
            ty: formal_type,
        }),
    );
    let root = ty(
        &mut f.source,
        TypeForm::Applied {
            declaration: nominal,
            arguments: vec![f.cell],
        },
    );
    let admission = OwnedAssociatedAdmission::new();
    AssociatedRead {
        read: &f.source,
        admission: &admission,
    }
    .walk(
        &[root],
        AssociatedTypeContext::Lexical(f.scheme),
        GENERATION,
        true,
    )
    .expect("stored-RHS field formal belongs to its exact raw nominal definition");

    let foreign_parameter = f.scheme_parameter;
    let foreign = ty(
        &mut f.source,
        TypeForm::TypeParameter {
            parameter: foreign_parameter,
        },
    );
    let OwnerRecord::Field(record) = f.source.owners.get_mut(&OwnerKey::Field(field)).unwrap()
    else {
        panic!("nominal field")
    };
    record.ty = foreign;
    let admission = OwnedAssociatedAdmission::new();
    let error = AssociatedRead {
        read: &f.source,
        admission: &admission,
    }
    .walk(
        &[root],
        AssociatedTypeContext::Lexical(f.scheme),
        GENERATION,
        true,
    )
    .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic);
}

#[test]
fn owned_associated_raw_nominal_field_and_case_generations_remain_exact() {
    for case_form in [false, true] {
        for (generation, expected) in [
            (0, "kernel_read_view_generation"),
            (GENERATION, "kernel_owned_associated"),
        ] {
            let mut f = fixture();
            let module = ModuleId::migrate(SEED, 22000);
            let scope_parameter = ReadScopeParameter {
                id: crate::platform::semantic_id::ReadScopeParameterId::migrate(SEED, 22000),
                name: name("source"),
            };
            let parameter = scope_parameter.id;
            let view = declaration(
                &mut f.source,
                module,
                22000,
                "HiddenView",
                OwnerKind::ReadView,
                DeclarationPayload::ReadView(ReadViewDeclaration {
                    type_parameters: vec![],
                    scope_parameter,
                    fields: vec![],
                }),
            );
            let view_type = ty(
                &mut f.source,
                TypeForm::ReadViewApplied {
                    declaration: view,
                    arguments: vec![],
                    read_scope: ReadScopeArgument::Parameter(ReadScopeReference {
                        binder: ReadScopeBinder::Declaration(view),
                        parameter,
                    }),
                },
            );
            let field = FieldId::migrate(SEED, 22000);
            let case = crate::platform::semantic_id::CaseId::migrate(SEED, 22000);
            let payload = if case_form {
                DeclarationPayload::Variant {
                    type_parameters: vec![],
                    cases: vec![case],
                }
            } else {
                DeclarationPayload::Record {
                    type_parameters: vec![],
                    fields: vec![field],
                }
            };
            let nominal = declaration(
                &mut f.source,
                module,
                22001,
                "Container",
                if case_form {
                    OwnerKind::Variant
                } else {
                    OwnerKind::Record
                },
                payload,
            );
            if case_form {
                let mut header = OwnerHeader::new(OwnerKey::Case(case), OwnerKind::Case);
                header.contract_version = generation;
                f.source.owners.insert(
                    OwnerKey::Case(case),
                    OwnerRecord::Case(CaseRecord {
                        header,
                        declaration: nominal.declaration,
                        name: name("value"),
                        payload: Some(view_type),
                    }),
                );
            } else {
                let mut header = OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field);
                header.contract_version = generation;
                f.source.owners.insert(
                    OwnerKey::Field(field),
                    OwnerRecord::Field(FieldRecord {
                        header,
                        declaration: nominal.declaration,
                        name: name("value"),
                        ty: view_type,
                    }),
                );
            }
            let root = ty(
                &mut f.source,
                TypeForm::Named {
                    declaration: nominal,
                },
            );
            let error = OwnedAssociatedAdmission::new()
                .validate_occurrences(
                    &f.source,
                    &[root],
                    AssociatedTypeContext::Forbidden,
                    GENERATION,
                )
                .unwrap_err();
            assert_eq!(error.class, DiagnosticClass::Semantic);
            assert_eq!(
                error.code, expected,
                "case={case_form} field_generation={generation}"
            );
        }
    }
}

#[test]
fn associated_default_metadata_is_unbounded_and_explicit_maximum_remains_finite() {
    let defaults = AssociatedAdmissionLimits::default();
    assert_eq!(defaults.maximum_steps, None);
    assert_eq!(defaults.maximum_metadata_bytes, None);
    let unbounded = OwnedAssociatedAdmission::new();
    unbounded.reserve_bytes(64 * 1_048_576 + 1).unwrap();
    assert!(!unbounded.metadata_observation().saturated);
    unbounded.reserve::<usize>(usize::MAX).unwrap();
    assert!(unbounded.metadata_observation().saturated);
    assert_eq!(unbounded.metadata_observation().consumed, usize::MAX);
    let finite = OwnedAssociatedAdmission::with_limits(AssociatedAdmissionLimits {
        maximum_metadata_bytes: Some(usize::MAX),
        ..defaults
    });
    finite.reserve_bytes(usize::MAX).unwrap();
    assert!(finite.reserve_bytes(1).is_err());
    assert_eq!(
        finite.metadata_observation(),
        ValidationWork::exact(usize::MAX)
    );
    let zero = OwnedAssociatedAdmission::with_limits(AssociatedAdmissionLimits {
        maximum_metadata_bytes: Some(0),
        ..defaults
    });
    zero.reserve_bytes(0).unwrap();
    assert!(zero.reserve_bytes(1).is_err());
}
