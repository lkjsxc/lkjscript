//! Bounded counterexamples for finite admission and exact lexical/child ownership.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use crate::platform::semantic_id::{
    BindingId, DeclarationId, ExpressionId, ModuleId, OperationId, ParameterId, PortId,
    RepositoryId, RevisionId, TargetId,
};
use std::cell::Cell;
use std::collections::BTreeMap;

const SEED: &[u8] = b"finite-kernel-admission";

fn name(value: &str) -> Name {
    Name::new(value).unwrap()
}

fn map(entries: usize) -> MapRoot {
    MapRoot::from_parts(
        PageDigest::from_bytes([1; 32]),
        entries as u64,
        MapContentDigest::from_bytes([2; 32]),
    )
}

fn refresh(snapshot: &mut KernelSnapshot) {
    snapshot.root.owners = map(snapshot.owners.len());
    snapshot.root.dependencies = map(snapshot.dependencies.len());
}

fn insert(snapshot: &mut KernelSnapshot, record: OwnerRecord) {
    assert!(snapshot.owners.insert(record.owner(), record).is_none());
    refresh(snapshot);
}

fn expression(
    snapshot: &mut KernelSnapshot,
    ordinal: u64,
    op: ExpressionOperation,
) -> ExpressionId {
    let id = ExpressionId::migrate(SEED, ordinal);
    insert(
        snapshot,
        OwnerRecord::Expression(ExpressionRecord::new(id, op).unwrap()),
    );
    id
}

fn ty(snapshot: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    snapshot.types.insert(digest, object);
    digest
}

fn fixture() -> KernelSnapshot {
    let mut snapshot = KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(SEED, 0),
            package_id: PackageId::migrate(SEED, 0),
            package_name: name("finite"),
            owners: map(0),
            dependencies: map(0),
            retirements: map(0),
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    };
    let module = ModuleId::migrate(SEED, 0);
    insert(
        &mut snapshot,
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: name("m"),
        }),
    );
    snapshot
}

fn declaration(
    snapshot: &mut KernelSnapshot,
    ordinal: u64,
    label: &str,
    kind: OwnerKind,
    payload: DeclarationPayload,
) -> DeclarationId {
    let id = DeclarationId::migrate(SEED, ordinal);
    insert(
        snapshot,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(id), kind),
            module: ModuleId::migrate(SEED, 0),
            name: name(label),
            visibility: DeclarationVisibility::Private,
            payload,
        }),
    );
    id
}

fn function(
    snapshot: &mut KernelSnapshot,
    ordinal: u64,
    label: &str,
    body: ExpressionId,
    result: TypeObjectDigest,
) -> DeclarationId {
    declaration(
        snapshot,
        ordinal,
        label,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(FunctionDeclaration {
            result_borrow: None,
            implementation_parameters: Vec::new(),
            requirement_parameters: Vec::new(),
            effect_parameters: Vec::new(),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            result,
            effect: FunctionEffect::Pure,
            body,
        }),
    )
}

fn binding(
    snapshot: &mut KernelSnapshot,
    ordinal: u64,
    value: ExpressionId,
    annotation: Option<TypeObjectDigest>,
) -> BindingId {
    let id = BindingId::migrate(SEED, ordinal);
    insert(
        snapshot,
        OwnerRecord::Binding(BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(id), OwnerKind::Binding),
            name: name(&format!("x{ordinal}")),
            kind: BindingKind::Let,
            value: Some(value),
            declared_type: annotation,
        }),
    );
    id
}

fn incremental(
    snapshot: &KernelSnapshot,
    roots: impl IntoIterator<Item = OwnerKey>,
    maximum_steps: usize,
) -> (Vec<Diagnostic>, usize) {
    let mut diagnostics = Vec::new();
    let mut work = 0;
    let result = validate_expression_roots_with_limits(
        snapshot,
        roots,
        &mut diagnostics,
        &mut work,
        ExpressionValidationLimits {
            maximum_steps,
            maximum_diagnostics: 32,
        },
    );
    assert_eq!(result, Ok(()), "unexpected exhaustion: {diagnostics:?}");
    (diagnostics, work)
}

fn has(errors: &[Diagnostic], code: &str) -> bool {
    errors.iter().any(|error| error.code == code)
}

#[test]
fn descendant_cycle_returns_its_semantic_rejection_with_finite_work() {
    let mut snapshot = fixture();
    let unit = ty(&mut snapshot, TypeForm::Unit);
    let descendant = ExpressionId::migrate(SEED, 1);
    expression(
        &mut snapshot,
        1,
        ExpressionOperation::Sequence {
            items: vec![descendant],
        },
    );
    let root = expression(
        &mut snapshot,
        0,
        ExpressionOperation::Sequence {
            items: vec![descendant],
        },
    );
    function(&mut snapshot, 0, "unused", root, unit);
    let errors = validate_full_with_limit(&snapshot, 512).unwrap_err();
    assert!(has(&errors, "kernel_full_expression_cycle"));
    assert!(!has(&errors, "kernel_full_work"));
}

#[test]
fn unused_interface_cannot_list_another_interfaces_operation() {
    let mut snapshot = fixture();
    let unit = ty(&mut snapshot, TypeForm::Unit);
    let operation = OperationId::migrate(SEED, 0);
    let a = declaration(
        &mut snapshot,
        0,
        "A",
        OwnerKind::Interface,
        DeclarationPayload::Interface {
            operations: vec![operation],
        },
    );
    insert(
        &mut snapshot,
        OwnerRecord::Operation(OperationRecord {
            header: OwnerHeader::new(OwnerKey::Operation(operation), OwnerKind::Operation),
            declaration: a,
            name: name("op"),
            parameters: Vec::new(),
            result: unit,
            idempotency: Idempotency::Idempotent,
            external_visibility: ExternalVisibility::None,
        }),
    );
    validate_full_with_limit(&snapshot, 1024).unwrap();
    declaration(
        &mut snapshot,
        1,
        "B",
        OwnerKind::Interface,
        DeclarationPayload::Interface {
            operations: vec![operation],
        },
    );
    assert!(has(
        &validate_full_with_limit(&snapshot, 1024).unwrap_err(),
        "kernel_full_parent_mismatch"
    ));
}

#[test]
fn incremental_rejects_cross_function_self_and_forward_binding_references() {
    for mode in ["foreign", "self", "forward"] {
        let mut snapshot = fixture();
        let unit = ty(&mut snapshot, TypeForm::Unit);
        let value = expression(&mut snapshot, 0, ExpressionOperation::Unit {});
        let x = binding(&mut snapshot, 0, value, Some(unit));
        let reference = expression(
            &mut snapshot,
            1,
            ExpressionOperation::Local {
                value: LocalValueReference::LexicalBinding(x),
            },
        );
        let body = if mode == "foreign" {
            let own_body = expression(&mut snapshot, 2, ExpressionOperation::Unit {});
            let own_let = expression(
                &mut snapshot,
                3,
                ExpressionOperation::Let {
                    bindings: vec![x],
                    body: own_body,
                },
            );
            function(&mut snapshot, 0, "owner", own_let, unit);
            reference
        } else if mode == "self" {
            snapshot.owners.remove(&OwnerKey::Expression(value));
            let OwnerRecord::Binding(record) =
                snapshot.owners.get_mut(&OwnerKey::Binding(x)).unwrap()
            else {
                panic!()
            };
            record.value = Some(reference);
            let body = expression(&mut snapshot, 2, ExpressionOperation::Unit {});
            expression(
                &mut snapshot,
                3,
                ExpressionOperation::Let {
                    bindings: vec![x],
                    body,
                },
            )
        } else {
            let y = binding(&mut snapshot, 1, reference, Some(unit));
            let body = expression(&mut snapshot, 2, ExpressionOperation::Unit {});
            expression(
                &mut snapshot,
                3,
                ExpressionOperation::Let {
                    bindings: vec![y, x],
                    body,
                },
            )
        };
        let f = function(&mut snapshot, 1, "invalid", body, unit);
        refresh(&mut snapshot);
        let (errors, _) = incremental(&snapshot, [OwnerKey::Declaration(f)], 1024);
        assert!(
            has(&errors, "kernel_full_lexical_scope"),
            "{mode}: {errors:?}"
        );
        assert!(has(
            &validate_full_with_limit(&snapshot, 2048).unwrap_err(),
            "kernel_full_lexical_scope"
        ));
    }
}

#[test]
fn operation_parameters_have_no_function_execution_environment() {
    let mut snapshot = fixture();
    let unit = ty(&mut snapshot, TypeForm::Unit);
    let operation = OperationId::migrate(SEED, 0);
    let parameter = ParameterId::migrate(SEED, 0);
    let interface = declaration(
        &mut snapshot,
        0,
        "I",
        OwnerKind::Interface,
        DeclarationPayload::Interface {
            operations: vec![operation],
        },
    );
    insert(
        &mut snapshot,
        OwnerRecord::Operation(OperationRecord {
            header: OwnerHeader::new(OwnerKey::Operation(operation), OwnerKind::Operation),
            declaration: interface,
            name: name("op"),
            parameters: vec![parameter],
            result: unit,
            idempotency: Idempotency::Idempotent,
            external_visibility: ExternalVisibility::None,
        }),
    );
    insert(
        &mut snapshot,
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Operation(operation),
            name: name("p"),
            ty: unit,
            use_mode: ParameterUse::Unrestricted,
            resource_requirement: None,
        }),
    );
    let body = expression(
        &mut snapshot,
        0,
        ExpressionOperation::Local {
            value: LocalValueReference::OperationParameter(parameter),
        },
    );
    let f = function(&mut snapshot, 1, "invalid", body, unit);
    assert!(has(
        &incremental(&snapshot, [OwnerKey::Declaration(f)], 1024).0,
        "kernel_type_parameter_scope"
    ));
    assert!(has(
        &validate_full_with_limit(&snapshot, 2048).unwrap_err(),
        "kernel_type_parameter_scope"
    ));
}

#[test]
fn branch_local_bindings_do_not_escape_into_a_sibling_branch() {
    let mut snapshot = fixture();
    let unit = ty(&mut snapshot, TypeForm::Unit);
    let value = expression(&mut snapshot, 0, ExpressionOperation::Unit {});
    let x = binding(&mut snapshot, 0, value, None);
    let left = expression(
        &mut snapshot,
        1,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(x),
        },
    );
    let left = expression(
        &mut snapshot,
        2,
        ExpressionOperation::Let {
            bindings: vec![x],
            body: left,
        },
    );
    let right = expression(
        &mut snapshot,
        3,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(x),
        },
    );
    let condition = expression(&mut snapshot, 4, ExpressionOperation::Bool { value: true });
    let root = expression(
        &mut snapshot,
        5,
        ExpressionOperation::If {
            condition,
            when_true: left,
            when_false: right,
        },
    );
    let f = function(&mut snapshot, 0, "branches", root, unit);
    assert!(has(
        &incremental(&snapshot, [OwnerKey::Declaration(f)], 1024).0,
        "kernel_full_lexical_scope"
    ));
    assert!(has(
        &validate_full_with_limit(&snapshot, 2048).unwrap_err(),
        "kernel_full_lexical_scope"
    ));
}

fn diamond_lets(
    annotated: bool,
) -> (
    KernelSnapshot,
    DeclarationId,
    ExpressionId,
    TypeObjectDigest,
) {
    let mut snapshot = fixture();
    let unit = ty(&mut snapshot, TypeForm::Unit);
    let first = expression(&mut snapshot, 0, ExpressionOperation::Unit {});
    let mut bindings = vec![binding(&mut snapshot, 0, first, annotated.then_some(unit))];
    for i in 1..=20_u64 {
        let previous = *bindings.last().unwrap();
        let condition = expression(
            &mut snapshot,
            i * 4,
            ExpressionOperation::Bool { value: true },
        );
        let left = expression(
            &mut snapshot,
            i * 4 + 1,
            ExpressionOperation::Local {
                value: LocalValueReference::LexicalBinding(previous),
            },
        );
        let right = expression(
            &mut snapshot,
            i * 4 + 2,
            ExpressionOperation::Local {
                value: LocalValueReference::LexicalBinding(previous),
            },
        );
        let value = expression(
            &mut snapshot,
            i * 4 + 3,
            ExpressionOperation::If {
                condition,
                when_true: left,
                when_false: right,
            },
        );
        bindings.push(binding(&mut snapshot, i, value, annotated.then_some(unit)));
    }
    let selected = expression(
        &mut snapshot,
        100,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(*bindings.last().unwrap()),
        },
    );
    let body = expression(
        &mut snapshot,
        101,
        ExpressionOperation::Let {
            bindings,
            body: selected,
        },
    );
    let f = function(&mut snapshot, 0, "diamond", body, unit);
    (snapshot, f, selected, unit)
}

#[test]
fn completed_let_types_are_reused_and_subexpression_inference_retains_scope() {
    let mut observations = Vec::new();
    for annotated in [false, true] {
        let (snapshot, f, selected, unit) = diamond_lets(annotated);
        let (errors, work) = incremental(&snapshot, [OwnerKey::Declaration(f)], 2048);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(work < 1024);
        observations.push(work);
        let mut work = 0;
        assert_eq!(
            infer_function_expression_type(
                &snapshot,
                f,
                selected,
                &FunctionEffect::Pure,
                &mut work,
                2048
            )
            .unwrap(),
            unit
        );
        validate_full_with_limit(&snapshot, 4096).unwrap();
    }
    assert_eq!(observations[0], observations[1]);
}

#[test]
fn resource_shape_dag_work_is_admitted_and_interruptible() {
    let mut snapshot = fixture();
    let mut child = ty(&mut snapshot, TypeForm::Unit);
    for _ in 0..18 {
        child = ty(
            &mut snapshot,
            TypeForm::StructuralRecord {
                fields: vec![
                    StructuralTypeField {
                        name: name("a"),
                        ty: child,
                    },
                    StructuralTypeField {
                        name: name("b"),
                        ty: child,
                    },
                ],
            },
        );
    }
    // Unreachable metadata produces a semantic error before inference; resource admission
    // still traverses it. No expression or nominal traversal can supply these checkpoints.
    let checkpoints = Cell::new(0);
    let checkpoint = || {
        checkpoints.set(checkpoints.get() + 1);
        if checkpoints.get() >= 100 {
            Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "finite_fixture_cancelled",
                "cancel resource traversal",
            ))
        } else {
            Ok(())
        }
    };
    let mut work = 0;
    let errors = validate_full_checked(&snapshot, 4096, &mut work, &checkpoint).unwrap_err();
    assert!(has(&errors, "finite_fixture_cancelled"));
    assert!(work < 100);
    let mut work = 0;
    let errors = super::validate::validate_full_observed(&snapshot, 4096, &mut work).unwrap_err();
    assert!(has(&errors, "kernel_full_type_unreachable"));
    assert!(!has(&errors, "kernel_full_work"));
    assert!(work < 4096);
}

#[test]
fn imported_target_checks_both_port_parent_and_component_inventory() {
    for mismatch in ["foreign", "parent", "inventory"] {
        let mut snapshot = fixture();
        let package = PackageId::migrate(SEED, 1);
        let revision = PackageRevisionDigest::from_bytes([3; 32]);
        let a = DeclarationId::migrate(SEED, 0);
        let b = DeclarationId::migrate(SEED, 1);
        let port = PortId::migrate(SEED, 0);
        let target = TargetId::migrate(SEED, 0);
        let unit = TypeObject::new(TypeForm::Unit).unwrap();
        let unit_digest = encode_type_object(&unit).unwrap().0;
        let callable = TypeObject::new(TypeForm::Function {
            parameters: Vec::new(),
            result: unit_digest,
        })
        .unwrap();
        let callable_digest = encode_type_object(&callable).unwrap().0;
        snapshot.dependency_types.insert(unit_digest, unit);
        snapshot.dependency_types.insert(callable_digest, callable);
        snapshot.dependencies.insert(
            package,
            DependencyRecord {
                graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
                package,
                semantic_revision: RevisionId::from_digest([4; 32]),
                package_revision: revision,
            },
        );
        let interface = BTreeMap::from([
            (
                OwnerKey::Declaration(a),
                PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                    header: OwnerHeader::new(OwnerKey::Declaration(a), OwnerKind::Component),
                    name: name("A"),
                    payload: PackageInterfaceDeclarationPayload::Component {
                        requirements: Vec::new(),
                        ports: if mismatch == "parent" {
                            vec![port]
                        } else {
                            Vec::new()
                        },
                    },
                }),
            ),
            (
                OwnerKey::Declaration(b),
                PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                    header: OwnerHeader::new(OwnerKey::Declaration(b), OwnerKind::Component),
                    name: name("B"),
                    payload: PackageInterfaceDeclarationPayload::Component {
                        requirements: Vec::new(),
                        ports: if mismatch == "foreign" {
                            vec![port]
                        } else {
                            Vec::new()
                        },
                    },
                }),
            ),
            (
                OwnerKey::Port(port),
                PackageInterfaceRecord::Port(PackageInterfacePort {
                    header: OwnerHeader::new(OwnerKey::Port(port), OwnerKind::Port),
                    declaration: if mismatch == "inventory" { a } else { b },
                    name: name("run"),
                    function_type: callable_digest,
                }),
            ),
        ]);
        snapshot.dependency_interfaces.insert(revision, interface);
        insert(
            &mut snapshot,
            OwnerRecord::Target(TargetRecord {
                header: OwnerHeader::new(OwnerKey::Target(target), OwnerKind::Target),
                name: name("target"),
                component: DeclarationReference {
                    package,
                    declaration: a,
                },
                port: Some(PortReference { package, port }),
                runner: crate::platform::package::RunnerKind::Command,
            }),
        );
        assert!(has(
            &incremental(&snapshot, [OwnerKey::Target(target)], 1024).0,
            "kernel_full_target_port_owner"
        ));
        assert!(has(
            &validate_full_with_limit(&snapshot, 2048).unwrap_err(),
            "kernel_full_target_port_owner"
        ));
        let owners = snapshot.dependency_interfaces.get_mut(&revision).unwrap();
        let PackageInterfaceRecord::Declaration(component) =
            owners.get_mut(&OwnerKey::Declaration(a)).unwrap()
        else {
            panic!()
        };
        component.payload = PackageInterfaceDeclarationPayload::Component {
            requirements: Vec::new(),
            ports: vec![port],
        };
        let PackageInterfaceRecord::Declaration(component) =
            owners.get_mut(&OwnerKey::Declaration(b)).unwrap()
        else {
            panic!()
        };
        component.payload = PackageInterfaceDeclarationPayload::Component {
            requirements: Vec::new(),
            ports: Vec::new(),
        };
        let PackageInterfaceRecord::Port(record) = owners.get_mut(&OwnerKey::Port(port)).unwrap()
        else {
            panic!()
        };
        record.declaration = a;
        assert!(
            incremental(&snapshot, [OwnerKey::Target(target)], 1024)
                .0
                .is_empty()
        );
        validate_full_with_limit(&snapshot, 2048).unwrap();
    }
}

#[test]
fn substituted_nominal_phantom_arguments_are_admitted_before_discard() {
    use crate::platform::control::decode_compact_change;
    use crate::platform::publication::GraphRepository;

    for operation in ["call", "function-value", "record"] {
        let temporary = tempfile::tempdir().unwrap();
        let created = GraphRepository::create(
            &temporary.path().join("meaning"),
            &super::tests::witness_snapshot(),
            None,
        )
        .unwrap();
        let repository = &created.repository;
        let source = |argument: &str| {
            let selected = match operation {
                "record" => {
                    format!("(record Marker (types {argument}) (field Marker::value (i64 0)))")
                }
                _ => format!("({operation} make (types {argument}))"),
            };
            format!(
                r#"request base={}
declarations.begin
(units
  (module create phantom-admission
    (record create Marker (visibility private)
      (type-parameter create T)
      (field create value (type I64)))
    (function create make (visibility private)
      (type-parameter create U)
      (returns (Marker U)) (effect pure)
      (body (record Marker (types U) (field Marker::value (i64 0)))))
    (function create discard (visibility private)
      (returns Unit) (effect pure)
      (body (sequence {selected} (unit))))))
declarations.end
"#,
                repository.view_current().unwrap().revision(),
            )
        };
        // This authored candidate never persists Marker<Stream<I64>> as a type:
        // call/function-value substitution alone constructs the forbidden type.
        let invalid =
            decode_compact_change("phantom-invalid", source("(stream I64)").as_bytes()).unwrap();
        let before = repository.view_current().unwrap().revision();
        let errors = repository
            .prepare_authored_change(&invalid.semantic, invalid.options)
            .unwrap_err();
        assert!(
            has(&errors, "kernel_type_nominal_resource"),
            "{operation}: {errors:?}"
        );
        assert_eq!(repository.view_current().unwrap().revision(), before);

        let valid = decode_compact_change("phantom-valid", source("I64").as_bytes()).unwrap();
        let prepared = repository
            .prepare_authored_change(&valid.semantic, valid.options)
            .unwrap();
        repository.publish(&prepared.publication).unwrap();
        let mut snapshot = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        validate_full_with_limit(&snapshot, 100_000).unwrap();
        let (function, body) = snapshot
            .owners
            .values()
            .find_map(|record| match record {
                OwnerRecord::Declaration(record) if record.name.as_str() == "discard" => {
                    let DeclarationPayload::Function(function) = &record.payload else {
                        return None;
                    };
                    Some((record.header.owner, function.body))
                }
                _ => None,
            })
            .unwrap();
        let OwnerRecord::Expression(body) = &snapshot.owners[&OwnerKey::Expression(body)] else {
            panic!("discard body")
        };
        let ExpressionOperation::Sequence { items } = &body.operation else {
            panic!("discard sequence")
        };
        let selected = items[0];
        assert!(incremental(&snapshot, [function], 100_000).0.is_empty());
        let i64_type = ty(&mut snapshot, TypeForm::I64);
        let stream = ty(&mut snapshot, TypeForm::Stream { item: i64_type });
        let OwnerRecord::Expression(selected) = snapshot
            .owners
            .get_mut(&OwnerKey::Expression(selected))
            .unwrap()
        else {
            panic!("selected initializer")
        };
        let arguments = match &mut selected.operation {
            ExpressionOperation::Call { type_arguments, .. }
            | ExpressionOperation::FunctionValue { type_arguments, .. }
            | ExpressionOperation::Record { type_arguments, .. } => type_arguments,
            _ => panic!("selected generic operation"),
        };
        *arguments = vec![stream];
        assert!(has(
            &incremental(&snapshot, [function], 100_000).0,
            "kernel_type_nominal_resource"
        ));
        assert!(has(
            &validate_full_with_limit(&snapshot, 100_000).unwrap_err(),
            "kernel_type_nominal_resource"
        ));
    }
}
