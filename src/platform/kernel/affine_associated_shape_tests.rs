//! Isolated affine shape controls; complete associated source admission stays gated.
use super::*;
use crate::platform::kernel::{
    AssociatedTypeBound, AssociatedTypeMember, DeclarationRecord, DeclarationVisibility,
    ExpressionRecord, ImplementationParameter, KernelSnapshot, Name, OwnedContract, OwnedMethod,
    OwnedMethodParameter, OwnerHeader, OwnerKind, TypeObject, TypeParameterConstraints,
    TypeParameterRecord, encode_type_object,
};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, ImplementationParameterId, MethodId, TypeParameterId,
};

const SEED: &[u8] = b"affine-associated-projection-shapes";

struct Fixture {
    source: KernelSnapshot,
    scope: DeclarationReference,
    parameter: OwnerKey,
    projection: TypeObjectDigest,
}

#[derive(Clone, Copy, Debug)]
enum Entry {
    Parameter,
    Signature,
}

fn name(text: &str) -> Name {
    Name::new(text).expect("canonical fixture name")
}

fn intern(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).expect("canonical fixture type");
    let digest = encode_type_object(&object)
        .expect("canonical fixture type bytes")
        .0;
    source.types.insert(digest, object);
    digest
}

fn fixture(mode: ParameterUse) -> Fixture {
    let mut source = super::super::tests::transport_snapshot();
    let module = source
        .owners
        .keys()
        .find_map(|owner| match owner {
            OwnerKey::Module(id) => Some(*id),
            _ => None,
        })
        .expect("fixture module");
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
    let self_type = intern(
        &mut source,
        TypeForm::TypeParameter {
            parameter: caller_self,
        },
    );
    let scalar = intern(&mut source, TypeForm::I64);
    source.owners.insert(
        OwnerKey::Declaration(contract_id),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(contract_id), OwnerKind::OwnedContract),
            module,
            name: name("AssociatedShapeContract"),
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
    let projection = intern(
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
            parent: super::super::ParameterParent::Function(scope_id),
            name: name("value"),
            ty: projection,
            use_mode: mode,
            resource_requirement: None,
        }),
    );
    let body = ExpressionId::migrate(SEED, 0);
    source.owners.insert(
        OwnerKey::Expression(body),
        OwnerRecord::Expression(
            ExpressionRecord::new(
                body,
                ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(parameter_id),
                },
            )
            .expect("canonical local expression"),
        ),
    );
    source.owners.insert(
        OwnerKey::Declaration(scope_id),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(scope_id), OwnerKind::PureFunction),
            module,
            name: name("associatedShapeForward"),
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
                result_borrow: (mode == ParameterUse::Borrow).then_some(parameter_id),
                effect: FunctionEffect::Pure,
                body,
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

fn shape(
    fixture: &Fixture,
    entry: Entry,
    maximum: usize,
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
) -> (Result<(), Diagnostic>, usize) {
    let mut used = 0;
    let result = {
        let meter = work::Meter::new(&mut used, maximum);
        let checked = super::super::infer::CheckedExpressionRead {
            read: &fixture.source,
            checkpoint,
        };
        let read = work::Read {
            inner: &checked,
            meter: &meter,
        };
        let mut validator = AffineValidator {
            read: &read,
            work: &meter,
            current_function: None,
            associated_admission: OwnedAssociatedAdmission::new(),
        };
        match entry {
            Entry::Parameter => validator.validate_owner_shape(
                fixture.parameter,
                fixture
                    .source
                    .owners
                    .get(&fixture.parameter)
                    .expect("parameter"),
            ),
            Entry::Signature => validator
                .resource_function_parameters(fixture.scope)
                .map(|parameters| assert!(parameters.is_empty())),
        }
    };
    (result, used)
}

fn healthy() -> Result<(), Diagnostic> {
    Ok(())
}

#[test]
fn affine_associated_shapes_accept_consume_and_borrow_at_parameter_and_signature_entries() {
    for mode in [ParameterUse::Consume, ParameterUse::Borrow] {
        let fixture = fixture(mode);
        for entry in [Entry::Parameter, Entry::Signature] {
            let (result, used) = shape(
                &fixture,
                entry,
                super::super::contract::MAXIMUM_VALIDATION_WORK,
                &healthy,
            );
            result.expect("exact lexical member is Owned");
            assert!(used > 0, "{entry:?}");
        }
        for root in [
            fixture.parameter,
            OwnerKey::Declaration(fixture.scope.declaration),
        ] {
            let mut diagnostics = Vec::new();
            let mut used = 0;
            validate_affine_roots_with_limits(
                &fixture.source,
                [root],
                &mut diagnostics,
                &mut used,
                ExpressionValidationLimits {
                    maximum_steps: super::super::contract::MAXIMUM_VALIDATION_WORK,
                    maximum_diagnostics: 16,
                },
            )
            .expect("actual affine root remains bounded");
            assert!(diagnostics.is_empty(), "{mode:?} {root:?}: {diagnostics:?}");
            assert!(used > 0);
        }
    }
}

#[test]
fn affine_associated_shapes_refuse_reinterned_scope_witness_and_child_header_mutants() {
    for mutation in 0..5 {
        let mut fixture = fixture(ParameterUse::Consume);
        if mutation < 2 {
            let mut form = fixture.source.types[&fixture.projection].form.clone();
            let TypeForm::AssociatedWitnessProjection {
                scope, parameter, ..
            } = &mut form
            else {
                panic!("projection");
            };
            if mutation == 0 {
                scope.declaration = DeclarationId::migrate(SEED, 99);
            } else {
                *parameter = ImplementationParameterId::migrate(SEED, 99);
            }
            let replacement = intern(&mut fixture.source, form);
            let OwnerRecord::Parameter(parameter) = fixture
                .source
                .owners
                .get_mut(&fixture.parameter)
                .expect("parameter")
            else {
                panic!("parameter");
            };
            parameter.ty = replacement;
        } else {
            let OwnerRecord::Parameter(parameter) = fixture
                .source
                .owners
                .get_mut(&fixture.parameter)
                .expect("parameter")
            else {
                panic!("parameter");
            };
            match mutation {
                2 => parameter.header.contract_version = 32,
                3 => {
                    parameter.header.owner = OwnerKey::Parameter(ParameterId::migrate(SEED, 99));
                }
                4 => parameter.header.kind = OwnerKind::Field,
                _ => unreachable!("fixed mutation inventory"),
            }
        }
        for entry in [Entry::Parameter, Entry::Signature] {
            let (result, used) = shape(
                &fixture,
                entry,
                super::super::contract::MAXIMUM_VALIDATION_WORK,
                &healthy,
            );
            let error = result.expect_err("exact occurrence authority cannot be inferred");
            assert_eq!(
                error.class,
                DiagnosticClass::Semantic,
                "{mutation} {entry:?}: {error:?}"
            );
            let expected = if mutation < 2 {
                "kernel_owned_associated"
            } else if mutation == 2 {
                "kernel_owned_associated_generation"
            } else if matches!(entry, Entry::Parameter) {
                "kernel_affine_parameter_shape"
            } else {
                "kernel_affine_function_parameter_parent"
            };
            assert_eq!(error.code, expected, "{mutation} {entry:?}: {error:?}");
            if mutation < 3 || matches!(entry, Entry::Signature) {
                assert!(used > 0, "charged lookup before semantic refusal");
            }
        }
    }
}

#[test]
fn affine_associated_shapes_keep_exact_work_late_cancellation_and_fresh_retry() {
    use std::cell::Cell;
    let combined_fixture = fixture(ParameterUse::Consume);
    let roots = [
        combined_fixture.parameter,
        OwnerKey::Declaration(combined_fixture.scope.declaration),
    ];
    let mut combined_diagnostics = Vec::new();
    let mut combined_used = 0;
    validate_affine_roots_with_limits(
        &combined_fixture.source,
        roots,
        &mut combined_diagnostics,
        &mut combined_used,
        ExpressionValidationLimits {
            maximum_steps: super::super::contract::MAXIMUM_VALIDATION_WORK,
            maximum_diagnostics: 16,
        },
    )
    .expect("healthy cumulative two-root admission");
    assert!(combined_diagnostics.is_empty() && combined_used > 1);
    for maximum in [combined_used, combined_used - 1] {
        let mut diagnostics = Vec::new();
        let mut used = 0;
        let result = validate_affine_roots_with_limits(
            &combined_fixture.source,
            roots,
            &mut diagnostics,
            &mut used,
            ExpressionValidationLimits {
                maximum_steps: maximum,
                maximum_diagnostics: 16,
            },
        );
        if maximum == combined_used {
            result.expect("exact cumulative allowance");
        } else {
            assert_eq!(result, Err(ExpressionValidationExhaustion::Steps));
        }
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(used, maximum);
    }
    for entry in [Entry::Parameter, Entry::Signature] {
        let fixture = fixture(ParameterUse::Consume);
        let checks = Cell::new(0);
        let count = || {
            checks.set(checks.get() + 1);
            Ok(())
        };
        let (result, used) = shape(
            &fixture,
            entry,
            super::super::contract::MAXIMUM_VALIDATION_WORK,
            &count,
        );
        result.expect("healthy baseline");
        assert!(used > 1 && checks.get() > 2);
        let (exact, exact_used) = shape(&fixture, entry, used, &healthy);
        exact.expect("exact affine allowance");
        assert_eq!(exact_used, used);
        let (short, short_used) = shape(&fixture, entry, used - 1, &healthy);
        let error = short.expect_err("one less affine step");
        assert_eq!(error.class, DiagnosticClass::Resource);
        assert_eq!(error.code, "kernel_affine_work");
        assert_eq!(short_used, used - 1);

        let stop = checks.get() - 1;
        let cancelled_checks = Cell::new(0);
        let cancel = || {
            cancelled_checks.set(cancelled_checks.get() + 1);
            if cancelled_checks.get() >= stop {
                Err(Diagnostic::new(
                    DiagnosticClass::Cancelled,
                    "affine_associated_cancelled",
                    "cancelled during a late exact occurrence lookup",
                ))
            } else {
                Ok(())
            }
        };
        let (cancelled, cancelled_used) = shape(
            &fixture,
            entry,
            super::super::contract::MAXIMUM_VALIDATION_WORK,
            &cancel,
        );
        assert_eq!(
            cancelled
                .expect_err("late cancellation retains its class")
                .class,
            DiagnosticClass::Cancelled,
        );
        assert_eq!(cancelled_checks.get(), stop);
        assert!(cancelled_used > 0 && cancelled_used <= used);
        shape(
            &fixture,
            entry,
            super::super::contract::MAXIMUM_VALIDATION_WORK,
            &healthy,
        )
        .0
        .expect("fresh healthy retry is unaffected by refusal");
    }
}
