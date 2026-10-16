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
                    result: crate::platform::kernel::CallableResult {
                        ty: scalar,
                        relation: crate::platform::kernel::ResultRelation::Value,
                    },
                    effect: FunctionEffect::Pure,

                    read_scope_parameters: Vec::new(),
                    read_scope_sources: Vec::new(),
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
                result: crate::platform::kernel::CallableResult {
                    ty: projection,
                    relation: match (mode == ParameterUse::Borrow).then_some(parameter_id) {
                        Some(source) => {
                            crate::platform::kernel::ResultRelation::ReadFrom { source }
                        }
                        None => crate::platform::kernel::ResultRelation::Value,
                    },
                },
                effect: FunctionEffect::Pure,
                body,

                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
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

// Owning views are memory handles, not capability-resource values. This source
// uses the native prepare/publication path; no admission token is fabricated.
const OWNING_VIEW_LET_SOURCE: &str = r#"declarations.begin
(units (module create affine-view-let
  (read-view create Span (visibility public)
    (scope-parameter read_scope_ca000000000000000000000000000001 s))
  (interface create Input (visibility public)
    (operation create read (returns I64) (idempotency idempotent) (external-visibility none)))
  (function create ready (visibility private) (returns Unit (value)) (effect pure) (body (unit)))
  (component create authority (visibility private)
    (requirement create io (interface Input) (operations Input::read) (limits (maximum_calls 1 calls)))
    (port create ready (type (function () Unit)) (function ready)))
  (function create make (visibility private) (effect pure)
    (scope-parameter read_scope_ca000000000000000000000000000002 s)
    (parameter create owner (type OwnedI64Cell) (use borrow))
    (scope-source owner (read-scope (declaration make) read_scope_ca000000000000000000000000000002))
    (returns (read-view-application Span (types)
      (read-scope (declaration make) read_scope_ca000000000000000000000000000002))
      (view-from owner (read-scope (declaration make) read_scope_ca000000000000000000000000000002)))
    (body (view-create Span (types)
      (scope (read-scope (declaration make) read_scope_ca000000000000000000000000000002)) (source owner))))
  (function create task-view (visibility private) (effect (task (requirement authority::io)))
    (parameter create owner (type OwnedI64Cell) (use borrow))
    (parameter create lease (type (resource Input)) (use borrow) (requirement authority::io))
    (returns Unit (value))
    (body (let
      (binding view (as $view-scope)
        (type (read-view-application Span (types) (lexical-read-scope task-view $view-scope)))
        (borrow-call (call make (scopes (lexical-read-scope task-view $view-scope)) (local owner))
          (binding constructed
            (type (read-view-application Span (types) (lexical-read-scope task-view $view-scope))))
          (in (view-move (local constructed)))))
      (in (sequence (view-drop (local view)) (unit))))))))
declarations.end
"#;

fn owning_view_affine_root_errors(source: &KernelSnapshot, root: OwnerKey) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut used = 0;
    validate_affine_roots_with_limits(
        source,
        [root],
        &mut diagnostics,
        &mut used,
        ExpressionValidationLimits {
            maximum_steps: super::super::contract::MAXIMUM_VALIDATION_WORK,
            maximum_diagnostics: 16,
        },
    )
    .expect("genuine owning-view root stays within existing work limits");
    assert!(used > 0);
    diagnostics
}

#[test]
fn owning_read_view_let_binding_preserves_memory_and_resource_authority() {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        OWNING_VIEW_LET_SOURCE,
    )
    .expect("genuine owning-view Task native prepare and publication");
    super::super::validate_full(&source).unwrap();
    let (root, function) = source
        .owners
        .iter()
        .find_map(|(key, owner)| match owner {
            OwnerRecord::Declaration(record) if record.name.as_str() == "task-view" => {
                let DeclarationPayload::Function(function) = &record.payload else {
                    unreachable!()
                };
                Some((*key, function.clone()))
            }
            _ => None,
        })
        .unwrap();
    let view_binding = source
        .owners
        .iter()
        .find_map(|(key, owner)| match owner {
            OwnerRecord::Binding(record)
                if record.name.as_str() == "view" && record.value.is_some() =>
            {
                assert_eq!(record.kind, BindingKind::ReadView);
                assert_eq!(record.header.owner, *key);
                assert_eq!(
                    record.header.contract_version,
                    crate::platform::canonical_schema::SCHEMA_EPOCH
                );
                Some(*key)
            }
            _ => None,
        })
        .unwrap();
    let OwnerRecord::Binding(view) = &source.owners[&view_binding] else {
        unreachable!()
    };
    let initializer = view.value.unwrap();
    let OwnerRecord::Expression(initializer_record) =
        &source.owners[&OwnerKey::Expression(initializer)]
    else {
        unreachable!()
    };
    let ExpressionOperation::BorrowCall { call, .. } = initializer_record.operation else {
        unreachable!()
    };
    let OwnerRecord::Expression(call_record) = &source.owners[&OwnerKey::Expression(call)] else {
        unreachable!()
    };
    let ExpressionOperation::Call { arguments, .. } = &call_record.operation else {
        unreachable!()
    };
    assert_eq!(arguments.len(), 1);
    let source_argument = arguments[0];
    let lease = function.parameters[1];
    let OwnerRecord::Parameter(lease_record) = &source.owners[&OwnerKey::Parameter(lease)] else {
        unreachable!()
    };
    assert!(lease_record.resource_requirement.is_some());
    let resource_type = lease_record.ty;
    let scalar = source
        .types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::I64).then_some(*ty))
        .unwrap();
    assert!(owning_view_affine_root_errors(&source, root).is_empty());

    // The resource walk may recognize this owning category only after the
    // existing memory proof. It never admits a loan, annotation or source tag.
    for fault in 0..9 {
        let mut changed = source.clone();
        let OwnerRecord::Binding(record) = changed.owners.get_mut(&view_binding).unwrap() else {
            unreachable!()
        };
        match fault {
            0 => record.kind = BindingKind::Let,
            1 => record.kind = BindingKind::ReadViewBorrow,
            2 => record.value = None,
            3 => record.declared_type = None,
            4 => record.declared_type = Some(scalar),
            5 => record.declared_type = Some(resource_type),
            6 => record.header.contract_version = 33,
            7 => {
                let OwnerRecord::Expression(record) = changed
                    .owners
                    .get_mut(&OwnerKey::Expression(initializer))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.operation = ExpressionOperation::Unit {};
            }
            8 => {
                // A genuine capability parameter cannot replace the selected
                // Owned source just because both parameters use Borrow.
                let OwnerRecord::Expression(record) = changed
                    .owners
                    .get_mut(&OwnerKey::Expression(source_argument))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.operation = ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(lease),
                };
            }
            _ => unreachable!(),
        }
        let errors = owning_view_affine_root_errors(&changed, root);
        assert!(!errors.is_empty(), "fault{fault} unexpectedly admitted");
        assert!(
            !errors
                .iter()
                .any(|e| e.code == "kernel_affine_binding_missing"),
            "fault{fault} must fail existing memory/source admission: {errors:?}"
        );
        // Exact occurrence/scope admission can reject before the later memory
        // category check; all semantic mutations and healthy inputs stay exact.
        let (code, message) = match fault {
            0 | 1 => (
                "kernel_read_view",
                "lexical scope is not an exact typed loan binding",
            ),
            2 => (
                "kernel_buffer_ownership",
                "missing memory binding initializer",
            ),
            3..=5 => (
                "kernel_buffer_ownership",
                "owning view initializer requires its exact positive binding category",
            ),
            6 => (
                "kernel_owner_header",
                "owner header contract or kind does not match its record",
            ),
            7 => (
                "kernel_buffer_ownership",
                "affine initializer did not produce its declared handle category",
            ),
            8 => (
                "kernel_buffer_ownership",
                "child read source requires an exact typed live owner or view",
            ),
            _ => unreachable!(),
        };
        assert!(
            errors.iter().any(|e| e.class == DiagnosticClass::Semantic
                && e.code == code
                && e.message == message),
            "fault{fault}: {errors:?}"
        );
        assert!(
            super::super::validate_full(&changed).is_err(),
            "fault{fault}"
        );
    }

    // Keep a real capability in the same Task. Copying it into an intermediate
    // expression must still reach the independent capability-resource refusal.
    let mut copied = source.clone();
    let copy = ExpressionId::migrate(b"owning-view-let-resource-copy", 0);
    let body = ExpressionId::migrate(b"owning-view-let-resource-copy", 1);
    assert!(
        copied
            .owners
            .insert(
                OwnerKey::Expression(copy),
                OwnerRecord::Expression(
                    ExpressionRecord::new(
                        copy,
                        ExpressionOperation::Local {
                            value: LocalValueReference::FunctionParameter(lease),
                        }
                    )
                    .unwrap()
                ),
            )
            .is_none()
    );
    let OwnerRecord::Expression(record) = copied
        .owners
        .get_mut(&OwnerKey::Expression(function.body))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::Let {
        body: original_body,
        ..
    } = &mut record.operation
    else {
        unreachable!()
    };
    let previous = *original_body;
    *original_body = body;
    assert!(
        copied
            .owners
            .insert(
                OwnerKey::Expression(body),
                OwnerRecord::Expression(
                    ExpressionRecord::new(
                        body,
                        ExpressionOperation::Sequence {
                            items: vec![copy, previous],
                        }
                    )
                    .unwrap()
                ),
            )
            .is_none()
    );
    // FullValidator checks logical flat-map counts before meaning. Keep this
    // private fault inventory coherent after adding its two Expression owners;
    // it is never published or granted a repository/admission seal.
    let private_owner_inventory = copied.root.owners;
    copied.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        private_owner_inventory.page(),
        u64::try_from(copied.owners.len()).expect("bounded private fault owner inventory"),
        private_owner_inventory.content(),
    );
    let expected_copy_message = format!(
        "expression {copy}: function parameter {lease} can be used only by an explicit borrow, consume, variant transfer, or match"
    );
    let errors = owning_view_affine_root_errors(&copied, root);
    assert!(
        errors.iter().any(|e| e.class == DiagnosticClass::Semantic
            && e.code == "kernel_affine_resource_copy"
            && e.message == expected_copy_message),
        "{errors:?}"
    );
    let errors = super::super::validate_full(&copied).unwrap_err();
    assert!(
        errors.iter().any(|e| e.class == DiagnosticClass::Semantic
            && e.code == "kernel_affine_resource_copy"
            && e.message == expected_copy_message),
        "{errors:?}"
    );
    assert!(owning_view_affine_root_errors(&source, root).is_empty());
    super::super::validate_full(&source).unwrap();
}

#[test]
fn owning_read_view_sibling_annotation_uses_its_initializer_scope() {
    // Keep the earlier genuine fixture unchanged. A later handle carries the
    // earlier view's scope, not a new scope merely because its binding is affine.
    let tail = "      (in (sequence (view-drop (local view)) (unit)))";
    assert_eq!(OWNING_VIEW_LET_SOURCE.matches(tail).count(), 1);
    let input = OWNING_VIEW_LET_SOURCE.replace(
        tail,
        r#"      (binding retained (as $retained-view)
        (type (read-view-application Span (types) (lexical-read-scope task-view $view-scope)))
        (view-reborrow (local view) (lexical-read-scope task-view $view-scope)))
      (binding future (as $future-view)
        (type (read-view-application Span (types) (lexical-read-scope task-view $view-scope)))
        (view-reborrow (local retained) (lexical-read-scope task-view $view-scope)))
      (in (sequence (view-drop (local view)) (view-drop (local retained))
        (view-drop (local future)) (unit)))"#,
    );
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&input)
            .expect("native sibling view preparation and publication");
    super::super::validate_full(&source)
        .expect("full source must place a sibling annotation at its initializer");
    let directory = tempfile::tempdir().unwrap();
    let repository = crate::platform::publication::GraphRepository::create(
        &directory.path().join("sibling-views"),
        &source,
        None,
    )
    .unwrap()
    .repository;
    let exported = repository
        .export_package_container()
        .expect("fresh full-source export keeps exact sibling loan context");
    let bytes = exported.container.encode().unwrap();
    let container = crate::platform::package_transport::source::PackageContainer::decode(
        &bytes,
        exported.container.root.transport,
    )
    .unwrap();
    let admitted = container.admit().unwrap();
    assert_eq!(admitted.container.encode().unwrap(), bytes);
    assert_eq!(
        admitted.packages[&container.root.package_revision]
            .snapshot
            .owners,
        source.owners
    );
    let raw = crate::platform::package_transport::oracle::reconstruct(&container).unwrap();
    assert_eq!(raw.snapshots[&source.root.package_id].owners, source.owners);

    let (function_id, body) = source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.name.as_str() == "task-view" =>
            {
                let DeclarationPayload::Function(function) = &record.payload else {
                    unreachable!()
                };
                Some((*id, function.body))
            }
            _ => None,
        })
        .unwrap();
    let binding = |name: &str| {
        source
            .owners
            .iter()
            .find_map(|(key, record)| match (key, record) {
                (OwnerKey::Binding(id), OwnerRecord::Binding(record))
                    if record.name.as_str() == name && record.value.is_some() =>
                {
                    Some(*id)
                }
                _ => None,
            })
            .unwrap()
    };
    let view = binding("view");
    let retained = binding("retained");
    let future = binding("future");
    let OwnerRecord::Expression(body_record) = &source.owners[&OwnerKey::Expression(body)] else {
        unreachable!()
    };
    let ExpressionOperation::Let { bindings, .. } = &body_record.operation else {
        unreachable!()
    };
    assert_eq!(bindings.as_slice(), [view, retained, future]);
    let OwnerRecord::Binding(retained_record) = &source.owners[&OwnerKey::Binding(retained)] else {
        unreachable!()
    };
    assert_eq!(retained_record.kind, BindingKind::ReadView);
    let initializer = retained_record.value.unwrap();
    let declared = retained_record.declared_type.unwrap();
    let TypeForm::ReadViewApplied { read_scope, .. } = source.types[&declared].form else {
        unreachable!()
    };
    assert_eq!(
        read_scope,
        super::super::ReadScopeArgument::Lexical {
            callable: DeclarationReference {
                package: source.root.package_id,
                declaration: function_id
            },
            binding: view,
        }
    );
    let OwnerRecord::Expression(initializer_record) =
        &source.owners[&OwnerKey::Expression(initializer)]
    else {
        unreachable!()
    };
    assert!(
        matches!(initializer_record.operation, ExpressionOperation::ReborrowReadView {
        value: LocalValueReference::LexicalBinding(parent), read_scope: scope,
    } if parent == view && scope == read_scope)
    );

    for fault in [
        "future-scope",
        "self-value",
        "before-parent",
        "ended-source",
    ] {
        // These are private raw snapshots, not container/admission seals. Flat
        // map counts stay coherent when a fault adds exact typed/owner records.
        let mut changed = source.clone();
        match fault {
            "future-scope" => {
                let future_scope = super::super::ReadScopeArgument::Lexical {
                    callable: DeclarationReference {
                        package: source.root.package_id,
                        declaration: function_id,
                    },
                    binding: future,
                };
                let mut object = source.types[&declared].clone();
                let TypeForm::ReadViewApplied { read_scope, .. } = &mut object.form else {
                    unreachable!()
                };
                *read_scope = future_scope;
                let digest = encode_type_object(&object).unwrap().0;
                assert!(changed.types.insert(digest, object).is_none());
                let OwnerRecord::Binding(record) = changed
                    .owners
                    .get_mut(&OwnerKey::Binding(retained))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.declared_type = Some(digest);
                let OwnerRecord::Expression(record) = changed
                    .owners
                    .get_mut(&OwnerKey::Expression(initializer))
                    .unwrap()
                else {
                    unreachable!()
                };
                let ExpressionOperation::ReborrowReadView { read_scope, .. } =
                    &mut record.operation
                else {
                    unreachable!()
                };
                *read_scope = future_scope;
            }
            "self-value" => {
                let OwnerRecord::Expression(record) = changed
                    .owners
                    .get_mut(&OwnerKey::Expression(initializer))
                    .unwrap()
                else {
                    unreachable!()
                };
                let ExpressionOperation::ReborrowReadView { value, .. } = &mut record.operation
                else {
                    unreachable!()
                };
                *value = LocalValueReference::LexicalBinding(retained);
            }
            "before-parent" => {
                let OwnerRecord::Expression(record) =
                    changed.owners.get_mut(&OwnerKey::Expression(body)).unwrap()
                else {
                    unreachable!()
                };
                let ExpressionOperation::Let { bindings, .. } = &mut record.operation else {
                    unreachable!()
                };
                bindings.swap(0, 1);
            }
            "ended-source" => {
                let drop = ExpressionId::migrate(b"sibling-view-expired-source", 0);
                let reborrow = ExpressionId::migrate(b"sibling-view-expired-source", 1);
                assert!(
                    changed
                        .owners
                        .insert(
                            OwnerKey::Expression(drop),
                            OwnerRecord::Expression(
                                ExpressionRecord::new(
                                    drop,
                                    ExpressionOperation::DropReadView {
                                        value: LocalValueReference::LexicalBinding(view)
                                    }
                                )
                                .unwrap(),
                            )
                        )
                        .is_none()
                );
                assert!(
                    changed
                        .owners
                        .insert(
                            OwnerKey::Expression(reborrow),
                            OwnerRecord::Expression(
                                ExpressionRecord::new(
                                    reborrow,
                                    initializer_record.operation.clone()
                                )
                                .unwrap(),
                            )
                        )
                        .is_none()
                );
                let OwnerRecord::Expression(record) = changed
                    .owners
                    .get_mut(&OwnerKey::Expression(initializer))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.operation = ExpressionOperation::Sequence {
                    items: vec![drop, reborrow],
                };
            }
            _ => unreachable!(),
        }
        let owners = changed.root.owners;
        changed.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
            owners.page(),
            changed.owners.len() as u64,
            owners.content(),
        );
        let errors = super::super::validate_full(&changed).unwrap_err();
        let (code, message) = match fault {
            "future-scope" | "before-parent" => (
                "kernel_read_view",
                "lexical read scope has no live containing loan",
            ),
            "self-value" => ("kernel_full_lexical_scope", "outside its scope"),
            "ended-source" => (
                "kernel_buffer_ownership",
                "view source local handle is no longer live",
            ),
            _ => unreachable!(),
        };
        assert!(
            errors
                .iter()
                .any(|error| error.class == DiagnosticClass::Semantic
                    && error.code == code
                    && error.message.contains(message)),
            "{fault}: {errors:?}"
        );
    }
    // Refusal does not poison a healthy fresh source admission.
    super::super::validate_full(&source).unwrap();
    assert!(container.admit().is_ok());
}
