//! Isolated resource-shape prerequisites; complete associated admission stays gated.
use super::*;
use crate::platform::kernel::{
    AssociatedTypeBound, AssociatedTypeMember, DeclarationRecord, DeclarationVisibility,
    FunctionDeclaration, ImplementationParameter, Name, OwnedContract, OwnedMethod,
    OwnedMethodParameter, OwnerHeader, TypeParameterConstraints, TypeParameterRecord,
};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, DeclarationId, ImplementationParameterId, MethodId, ParameterId,
    TypeParameterId,
};

const SEED: &[u8] = b"full-associated-parameter-shape";

struct Fixture {
    source: KernelSnapshot,
    scope: DeclarationReference,
    parameter: OwnerKey,
    projection: TypeObjectDigest,
}

fn name(text: &str) -> Name {
    Name::new(text).expect("canonical name")
}

fn ty(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).expect("canonical type");
    let digest = encode_type_object(&object).expect("canonical type bytes").0;
    source.types.insert(digest, object);
    digest
}

fn fixture(use_mode: ParameterUse) -> Fixture {
    let mut source = super::super::tests::transport_snapshot();
    let module = source
        .owners
        .keys()
        .find_map(|key| match key {
            OwnerKey::Module(id) => Some(*id),
            _ => None,
        })
        .expect("canonical module");
    let contract_id = DeclarationId::migrate(SEED, 0);
    let scope_id = DeclarationId::migrate(SEED, 1);
    let contract = DeclarationReference {
        package: source.root.package_id,
        declaration: contract_id,
    };
    let scope = DeclarationReference {
        package: source.root.package_id,
        declaration: scope_id,
    };
    let member = AssociatedTypeMemberId::migrate(SEED, 0);
    let witness = ImplementationParameterId::migrate(SEED, 0);
    let contract_self = TypeParameterId::migrate(SEED, 0);
    let caller_self = TypeParameterId::migrate(SEED, 1);
    for (parameter, declaration) in [(contract_self, contract_id), (caller_self, scope_id)] {
        source.owners.insert(
            OwnerKey::TypeParameter(parameter),
            OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(parameter),
                    OwnerKind::TypeParameter,
                ),
                declaration,
                name: name("Self"),
                constraints: TypeParameterConstraints::Owned,
            }),
        );
    }
    let self_type = ty(
        &mut source,
        TypeForm::TypeParameter {
            parameter: caller_self,
        },
    );
    let scalar = ty(&mut source, TypeForm::I64);
    source.owners.insert(
        OwnerKey::Declaration(contract_id),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(contract_id), OwnerKind::OwnedContract),
            module,
            name: name("Output"),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::OwnedContract(OwnedContract {
                self_parameter: contract_self,
                type_parameters: vec![],
                associated_types: vec![AssociatedTypeMember {
                    id: member,
                    name: name("Out"),
                    bound: AssociatedTypeBound::Owned,
                }],
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
            }),
        }),
    );
    let projection = ty(
        &mut source,
        TypeForm::AssociatedWitnessProjection {
            scope,
            parameter: witness,
            contract,
            member,
        },
    );
    let parameter_id = ParameterId::migrate(SEED, 0);
    let parameter = OwnerKey::Parameter(parameter_id);
    source.owners.insert(
        parameter,
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(parameter, OwnerKind::Parameter),
            parent: ParameterParent::Function(scope_id),
            name: name("value"),
            ty: projection,
            use_mode,
            resource_requirement: None,
        }),
    );
    source.owners.insert(
        OwnerKey::Declaration(scope_id),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(scope_id), OwnerKind::PureFunction),
            module,
            name: name("forward"),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: vec![caller_self],
                implementation_parameters: vec![ImplementationParameter {
                    id: witness,
                    name: name("output"),
                    contract,
                    self_type,
                    type_arguments: vec![],
                }],
                requirement_parameters: vec![],
                effect_parameters: vec![],
                parameters: vec![parameter_id],
                result: projection,
                result_borrow: (use_mode == ParameterUse::Borrow).then_some(parameter_id),
                effect: FunctionEffect::Pure,
                body: ExpressionId::migrate(SEED, 0),
            }),
        }),
    );
    Fixture {
        source,
        scope,
        parameter,
        projection,
    }
}

fn validator<'a>(
    source: &'a KernelSnapshot,
    maximum_work: usize,
    checkpoint: &'a dyn Fn() -> Result<(), Diagnostic>,
) -> FullValidator<'a> {
    FullValidator {
        snapshot: source,
        read: super::super::infer::CheckedExpressionRead {
            read: source,
            checkpoint,
        },
        interrupted: false,
        diagnostics: Vec::new(),
        work: 0,
        maximum_work,
        associated_admission: super::super::owned_associated::OwnedAssociatedAdmission::new(),
        relations: Vec::new(),
        expression_parents: BTreeMap::new(),
        expression_root_owners: BTreeMap::new(),
        binding_containers: BTreeMap::new(),
    }
}

fn healthy() -> Result<(), Diagnostic> {
    Ok(())
}

#[test]
fn full_associated_parameter_shapes_accept_consume_and_borrow_without_relaxing_signature_use() {
    for mode in [ParameterUse::Consume, ParameterUse::Borrow] {
        let f = fixture(mode);
        let mut validator = validator(&f.source, MAXIMUM_VALIDATION_WORK, &healthy);
        validator.validate_resource_shapes();
        assert!(
            validator.diagnostics.is_empty(),
            "{:?}",
            validator.diagnostics
        );
        let record = f.source.owners.get(&f.parameter).expect("parameter");
        super::super::memory::validate_owner_at(
            &f.source,
            f.parameter,
            record,
            &validator.associated_admission,
        )
        .expect("exact Owned signature remains valid");
    }
    let f = fixture(ParameterUse::Unrestricted);
    let mut validator = validator(&f.source, MAXIMUM_VALIDATION_WORK, &healthy);
    let record = f.source.owners.get(&f.parameter).expect("parameter");
    assert!(
        validator
            .parameter_owns_storage(f.parameter, record)
            .expect("category")
    );
    let error = super::super::memory::validate_owner_at(
        &f.source,
        f.parameter,
        record,
        &validator.associated_admission,
    )
    .expect_err("category cannot grant an unrestricted use");
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert!(error.message.contains("explicit use"));
}

#[test]
fn full_associated_parameter_shapes_reject_foreign_missing_scope_and_old_owner_generations() {
    for mutation in 0..5 {
        let mut f = fixture(ParameterUse::Consume);
        match mutation {
            0 => {
                let mut form = f
                    .source
                    .types
                    .get(&f.projection)
                    .expect("projection")
                    .form
                    .clone();
                let TypeForm::AssociatedWitnessProjection { scope, .. } = &mut form else {
                    panic!("projection")
                };
                scope.package = PackageId::migrate(SEED, 2);
                let changed = ty(&mut f.source, form);
                let OwnerRecord::Parameter(record) =
                    f.source.owners.get_mut(&f.parameter).expect("parameter")
                else {
                    panic!("parameter")
                };
                record.ty = changed;
            }
            1 => {
                f.source
                    .owners
                    .remove(&OwnerKey::Declaration(f.scope.declaration));
            }
            2 => {
                let OwnerRecord::Parameter(record) =
                    f.source.owners.get_mut(&f.parameter).expect("parameter")
                else {
                    panic!("parameter")
                };
                record.header.contract_version = 32;
            }
            3 => {
                let OwnerRecord::Declaration(record) = f
                    .source
                    .owners
                    .get_mut(&OwnerKey::Declaration(f.scope.declaration))
                    .expect("function")
                else {
                    panic!("function")
                };
                record.header.contract_version = 32;
            }
            _ => {
                let OwnerRecord::Parameter(record) =
                    f.source.owners.get_mut(&f.parameter).expect("parameter")
                else {
                    panic!("parameter")
                };
                record.header.owner = OwnerKey::Parameter(ParameterId::migrate(SEED, 99));
            }
        }
        let mut validator = validator(&f.source, MAXIMUM_VALIDATION_WORK, &healthy);
        let record = f.source.owners.get(&f.parameter).expect("parameter");
        assert!(
            validator
                .parameter_owns_storage(f.parameter, record)
                .is_err(),
            "mutation {mutation}"
        );
        if mutation != 4 {
            assert!(
                validator.work > 0,
                "refused authority retains consumed work"
            );
        }
    }
}

#[test]
fn full_associated_parameter_shapes_forbid_operation_occurrences_and_preserve_resource_rules() {
    let mut f = fixture(ParameterUse::Consume);
    let OwnerRecord::Parameter(record) = f.source.owners.get_mut(&f.parameter).expect("parameter")
    else {
        panic!("parameter")
    };
    record.parent =
        ParameterParent::Operation(crate::platform::semantic_id::OperationId::migrate(SEED, 0));
    let mut operation_validator = validator(&f.source, MAXIMUM_VALIDATION_WORK, &healthy);
    assert!(
        operation_validator
            .parameter_owns_storage(
                f.parameter,
                f.source.owners.get(&f.parameter).expect("parameter")
            )
            .is_err()
    );

    for use_mode in [ParameterUse::Consume, ParameterUse::Unrestricted] {
        let mut f = fixture(use_mode);
        let resource = ty(
            &mut f.source,
            TypeForm::CapabilityResource { interface: f.scope },
        );
        let OwnerRecord::Parameter(record) =
            f.source.owners.get_mut(&f.parameter).expect("parameter")
        else {
            panic!("parameter")
        };
        record.ty = resource;
        let mut validator = validator(&f.source, MAXIMUM_VALIDATION_WORK, &healthy);
        validator.validate_resource_shapes();
        assert!(
            validator
                .diagnostics
                .iter()
                .any(|error| error.code == "kernel_affine_function_resource_requirement")
        );
        assert_eq!(
            validator
                .diagnostics
                .iter()
                .any(|error| error.code == "kernel_affine_function_resource_use"),
            use_mode == ParameterUse::Unrestricted
        );
    }
}

#[test]
fn full_associated_parameter_shapes_preserve_work_cancellation_and_fresh_retry() {
    let f = fixture(ParameterUse::Consume);
    let record = f.source.owners.get(&f.parameter).expect("parameter");
    let mut baseline = validator(&f.source, MAXIMUM_VALIDATION_WORK, &healthy);
    assert!(
        baseline
            .parameter_owns_storage(f.parameter, record)
            .expect("baseline")
    );
    let work = baseline.work;
    assert!(work > 1);
    let mut exact = validator(&f.source, work, &healthy);
    assert!(
        exact
            .parameter_owns_storage(f.parameter, record)
            .expect("exact work")
    );
    let mut short = validator(&f.source, work - 1, &healthy);
    assert_eq!(
        short
            .parameter_owns_storage(f.parameter, record)
            .expect_err("one less work")
            .class,
        DiagnosticClass::Resource
    );
    assert!(short.exhausted());
    let mut shared = validator(&f.source, work, &healthy);
    assert!(
        shared
            .parameter_owns_storage(f.parameter, record)
            .expect("first occurrence")
    );
    assert_eq!(
        shared
            .parameter_owns_storage(f.parameter, record)
            .expect_err("shared budget")
            .class,
        DiagnosticClass::Resource
    );
    let checks = std::cell::Cell::new(0);
    let cancel = || {
        checks.set(checks.get() + 1);
        if checks.get() == 2 {
            Err(Diagnostic::new(
                DiagnosticClass::Cancelled,
                "parameter_cancelled",
                "cancelled during exact occurrence lookup",
            ))
        } else {
            Ok(())
        }
    };
    let mut cancelled = validator(&f.source, MAXIMUM_VALIDATION_WORK, &cancel);
    let error = cancelled
        .parameter_owns_storage(f.parameter, record)
        .expect_err("cancelled owner read");
    assert_eq!(error.code, "parameter_cancelled");
    assert!(cancelled.interrupted);
    assert_eq!(cancelled.work, 1, "work before the refusal is retained");
    let mut retry = validator(&f.source, MAXIMUM_VALIDATION_WORK, &healthy);
    assert!(
        retry
            .parameter_owns_storage(f.parameter, record)
            .expect("fresh retry")
    );
}

#[test]
fn full_associated_parameter_shapes_keep_ordinary_historical_unrestricted_parameters() {
    for generation in [14, 33] {
        let mut f = fixture(ParameterUse::Unrestricted);
        let scalar = ty(&mut f.source, TypeForm::I64);
        let OwnerRecord::Parameter(parameter) =
            f.source.owners.get_mut(&f.parameter).expect("parameter")
        else {
            panic!("parameter")
        };
        parameter.header.contract_version = generation;
        parameter.ty = scalar;
        let OwnerRecord::Declaration(declaration) = f
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(f.scope.declaration))
            .expect("function")
        else {
            panic!("function")
        };
        declaration.header.contract_version = generation;
        let DeclarationPayload::Function(function) = &mut declaration.payload else {
            panic!("function")
        };
        function.type_parameters.clear();
        function.implementation_parameters.clear();
        function.result = scalar;
        let mut validator = validator(&f.source, MAXIMUM_VALIDATION_WORK, &healthy);
        let record = f.source.owners.get(&f.parameter).expect("parameter");
        assert!(
            !validator
                .parameter_owns_storage(f.parameter, record)
                .expect("ordinary category")
        );
        validator.validate_resource_shapes();
        assert!(
            validator.diagnostics.is_empty(),
            "generation {generation}: {:?}",
            validator.diagnostics
        );
    }
}
