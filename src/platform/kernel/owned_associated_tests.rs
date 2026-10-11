//! Point-reader adversaries for associated authority; no normalized producer input.
use super::*;
use crate::platform::semantic_id::{DeclarationId, ExpressionId, FieldId, MethodId, ModuleId};

const SEED: &[u8] = b"owned-associated-admission-tests";
const GENERATION: u16 = 33;

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
        result,
        result_borrow: None,
        effect: FunctionEffect::Pure,
        body: ExpressionId::migrate(SEED, 0),
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
            result: scalar,
            result_borrow: None,
            effect: FunctionEffect::Pure,
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
    let other_contract = declaration(
        &mut source,
        module,
        1,
        "OtherContract",
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(contract_value),
    );
    let scheme_parameter = TypeParameterId::migrate(SEED, 1);
    let mut bindings = vec![
        AssociatedTypeBinding { member, ty: cell },
        AssociatedTypeBinding {
            member: spare,
            ty: cell,
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
            .ty = symbolic;
    }
    let mut concrete_value = implementation;
    concrete_value.type_parameters.clear();
    concrete_value
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == member)
        .expect("Out binding")
        .ty = buffer;
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
    contract.methods[0].result = missing;
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
        .ty = scalar;
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
        .ty = hidden;
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
        .ty = foreign_parameter;
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
        .ty = nominal;
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
        .ty;
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
        maximum_steps: steps,
        maximum_metadata_bytes: bytes,
        maximum_depth: 256,
    };
    OwnedAssociatedAdmission::with_limits(limits)
        .validate_bindings(&f.source, f.scheme, &value)
        .expect("exact admission fit");
    for limits in [
        AssociatedAdmissionLimits {
            maximum_steps: steps - 1,
            ..limits
        },
        AssociatedAdmissionLimits {
            maximum_metadata_bytes: bytes - 1,
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
        .ty = public_type;
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
        contract_operand,
        implementations,
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
