//! Public authoring and independently derived rejection of structured child calls.
use super::*;

#[path = "parallel_result_tests.rs"]
mod results;

const SOURCE: &str = r#"declarations.begin
(units (module create parallel-proof
  (function create child (visibility public)
    (parameter create payload (type ByteBuffer) (use consume))
    (returns I64) (effect (task)) (body (i64 7)))
  (function create pair (visibility public)
    (parameter create a (type ByteBuffer) (use consume))
    (parameter create b (type ByteBuffer) (use consume))
    (returns (record (left I64) (right I64))) (effect (task))
    (body (parallel (call child (local a)) (call child (local b)))))))
declarations.end
"#;

fn author(source: &str) -> Result<KernelSnapshot, String> {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source)
}

fn child(source: &KernelSnapshot) -> DeclarationReference {
    let declaration = source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Declaration(d) if d.name.as_str() == "child" => match d.header.owner {
                OwnerKey::Declaration(id) => Some(id),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    DeclarationReference {
        package: source.root.package_id,
        declaration,
    }
}

fn branches(
    source: &KernelSnapshot,
) -> (
    crate::platform::semantic_id::ExpressionId,
    crate::platform::semantic_id::ExpressionId,
) {
    source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(e) => match e.operation {
                ExpressionOperation::Parallel { left, right } => Some((left, right)),
                _ => None,
            },
            _ => None,
        })
        .unwrap()
}

#[test]
fn parallel_native_meaning_has_two_owned_call_sites_and_closed_results() {
    let source = author(SOURCE).unwrap();
    assert!(memory_reference::accepts(&source));
    let (left, right) = branches(&source);
    assert_ne!(left, right);
    for expression in [left, right] {
        let call = parallel::admit_call(&source, expression).unwrap();
        assert_eq!(call.function, child(&source));
        assert_eq!(call.parameters.len(), 1);
        assert_eq!(call.parameters[0].use_mode, ParameterUse::Consume);
        assert!(matches!(source.types[&call.result].form, TypeForm::I64));
    }
    for record in source.owners.values() {
        if let OwnerRecord::Expression(e) = record
            && matches!(e.operation, ExpressionOperation::Parallel { .. })
        {
            let children = e.children();
            assert_eq!(
                children.iter().map(|child| child.role).collect::<Vec<_>>(),
                vec![
                    ExpressionChildRole::ParallelLeft,
                    ExpressionChildRole::ParallelRight
                ]
            );
            let mut disguised = e.clone();
            disguised.contract_version = 20;
            assert_eq!(
                encode_owner(&OwnerRecord::Expression(disguised))
                    .unwrap_err()
                    .code,
                "kernel_parallel_generation"
            );
        }
    }
}

#[test]
fn parallel_rehashed_predecessor_envelopes_and_mutated_graph_generations_reject() {
    let source = author(SOURCE).unwrap();
    let (key, original) = source.owners.iter().find_map(|(key, owner)| {
        matches!(owner, OwnerRecord::Expression(e) if matches!(e.operation, ExpressionOperation::Parallel { .. }))
            .then_some((*key, owner.clone()))
    }).unwrap();
    let (digest, bytes) = encode_owner(&original).unwrap();
    assert_eq!(
        decode_owner(&bytes, key, original.kind(), digest).unwrap(),
        original
    );
    for (generation, magic, domain) in [
        (18, *b"LKJOWN18", "lkjscript.kernel.owner-envelope.v18"),
        (19, *b"LKJOWN19", "lkjscript.kernel.owner-envelope.v19"),
        (20, *b"LKJOWN20", "lkjscript.kernel.owner-envelope.v20"),
    ] {
        let OwnerRecord::Expression(mut expression) = original.clone() else {
            panic!("parallel owner")
        };
        expression.contract_version = generation;
        let disguised = OwnerRecord::Expression(expression);
        let bytes = crate::platform::packed::encode(
            magic,
            domain,
            &disguised,
            contract::MAXIMUM_OWNER_OBJECT_BYTES,
        )
        .unwrap();
        let digest = OwnerObjectDigest::of(&bytes);
        assert_eq!(
            decode_owner(&bytes, key, disguised.kind(), digest)
                .unwrap_err()
                .code,
            "kernel_parallel_generation"
        );
        let mut forged = source.clone();
        forged.owners.insert(key, disguised);
        assert!(
            validate_full(&forged)
                .unwrap_err()
                .iter()
                .any(|d| d.code == "kernel_parallel_generation")
        );
        assert!(!memory_reference::accepts(&forged));
    }
    let mut older_root = source;
    older_root.root.graph_contract_version = 20;
    assert!(
        validate_full(&older_root)
            .unwrap_err()
            .iter()
            .any(|d| d.code == "kernel_owner_graph_generation")
    );
    assert!(!memory_reference::accepts(&older_root));
}

#[test]
fn parallel_rejects_duplicate_consumption_pure_parents_and_nondirect_children() {
    for (source, reason) in [
        (
            SOURCE.replace("(call child (local b))", "(call child (local a))"),
            "kernel_buffer_ownership",
        ),
        (
            SOURCE.replace(
                "(right I64))) (effect (task))",
                "(right I64))) (effect pure)",
            ),
            "kernel_parallel_context",
        ),
        (
            SOURCE.replace(
                "(call child (local b))",
                "(sequence (call child (local b)))",
            ),
            "kernel_parallel_call",
        ),
        (
            SOURCE.replace(
                "(returns I64) (effect (task))",
                "(returns I64) (effect pure)",
            ),
            "kernel_parallel_call",
        ),
    ] {
        let failure = author(&source).unwrap_err();
        assert!(failure.contains(reason), "{reason}: {failure}");
    }
    let mut source = author(SOURCE).unwrap();
    let (left, right) = branches(&source);
    let OwnerRecord::Expression(left) = source.owners[&OwnerKey::Expression(left)].clone() else {
        panic!("left call")
    };
    let ExpressionOperation::Call { arguments, .. } = left.operation else {
        panic!("left call")
    };
    let OwnerRecord::Expression(right) =
        source.owners.get_mut(&OwnerKey::Expression(right)).unwrap()
    else {
        panic!("right call")
    };
    let ExpressionOperation::Call {
        arguments: right_args,
        ..
    } = &mut right.operation
    else {
        panic!("right call")
    };
    *right_args = arguments;
    assert!(
        !memory_reference::accepts(&source),
        "independent owned oracle accepted a double move"
    );
}

#[test]
fn parallel_independent_admission_rejects_hidden_authority_and_task_shape_changes() {
    let source = author(SOURCE).unwrap();
    let target = child(&source);
    let (left, _) = branches(&source);
    for forbidden in [
        TypeForm::Secret,
        TypeForm::Function {
            parameters: Vec::new(),
            result: parallel::admit_call(&source, left).unwrap().result,
        },
        TypeForm::List {
            item: encode_type_object(&TypeObject::new(TypeForm::Secret).unwrap())
                .unwrap()
                .0,
        },
    ] {
        let mut forged = source.clone();
        let secret = TypeObject::new(TypeForm::Secret).unwrap();
        forged
            .types
            .insert(encode_type_object(&secret).unwrap().0, secret);
        let object = TypeObject::new(forbidden).unwrap();
        let ty = encode_type_object(&object).unwrap().0;
        forged.types.insert(ty, object);
        let OwnerRecord::Declaration(d) = forged
            .owners
            .get_mut(&OwnerKey::Declaration(target.declaration))
            .unwrap()
        else {
            panic!("child")
        };
        let DeclarationPayload::Function(f) = &mut d.payload else {
            panic!("child")
        };
        f.result = ty;
        assert!(parallel::admit_call(&forged, left).is_err());
        assert!(!memory_reference::accepts(&forged));
    }
    let mut forged = source.clone();
    let OwnerRecord::Declaration(d) = forged
        .owners
        .get_mut(&OwnerKey::Declaration(target.declaration))
        .unwrap()
    else {
        panic!("child")
    };
    let DeclarationPayload::Function(f) = &mut d.payload else {
        panic!("child")
    };
    f.effect = FunctionEffect::Pure;
    assert!(parallel::admit_call(&forged, left).is_err());
    assert!(!memory_reference::accepts(&forged));
}

#[test]
fn parallel_and_invocation_extraction_preserves_task_kind_and_exact_rows() {
    use crate::platform::builtin_standard::BuiltinStandard;
    use crate::platform::project_creation::{ProjectTemplate, create_project};
    use crate::platform::publication::GraphRepository;
    let standard = BuiltinStandard::load().unwrap();
    for (selected_source, result_type, expected_kind, captures, uses_clock) in [
        (
            "(parallel (call child (i64 17)) (call child (i64 -29)))",
            "(record (left I64) (right I64))",
            "task",
            "",
            false,
        ),
        ("(call child (i64 17))", "I64", "task", "", false),
        ("(i64 17)", "I64", "pure", "", false),
        (
            "(invoke (function-value child) (i64 17))",
            "I64",
            "task",
            "",
            false,
        ),
        (
            "(invoke (bind (function-value child) (i64 17)))",
            "I64",
            "task",
            "",
            false,
        ),
        (
            "(invoke (local callback) (i64 17))",
            "I64",
            "task",
            "(parameter create callback (type (task-function (I64) I64 (row))))",
            false,
        ),
        (
            "(invoke (function-value pure-child) (i64 17))",
            "I64",
            "pure",
            "",
            false,
        ),
        (
            "(function-value child)",
            "(task-function (I64) I64 (row))",
            "pure",
            "",
            false,
        ),
        (
            "(invoke (function-value read-clock))",
            "I64",
            "task",
            "",
            true,
        ),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let project = temporary.path().join("project");
        create_project(&project, "extraction-fixture", ProjectTemplate::Minimal).unwrap();
        let repository = GraphRepository::open(&project).unwrap();
        repository
            .stage_package_transport(standard.package_transport, standard.transport_bytes())
            .unwrap();
        // Naming the builtin selects exact references, not dependency authority.
        // Select its staged transport through the same authored operation as callers.
        let literal = format!(
            r#"request base={}
add.dependency package={} semantic-revision={} package-revision={}
declarations.begin
(units (use std builtin) (module create extraction
  (function create available (visibility private) (returns I64) (effect pure) (body (i64 0)))
  (component create authority (visibility public)
    (requirement create clock (interface std::WallClock)
      (operations std::WallClock::utc-milliseconds) (limits (maximum_calls 1 calls)))
    (requirement create unused (interface std::WallClock)
      (operations std::WallClock::utc-milliseconds) (limits (maximum_calls 1 calls)))
    (port create ready (type (function () I64)) (function available)))
  (function create read-clock (visibility public) (returns I64)
    (effect (task (requirement authority::clock)))
    (body (capability-call authority::clock std::WallClock::utc-milliseconds)))
  (function create child (visibility public) (parameter create n (type I64))
    (returns I64) (effect (task)) (body (local n)))
  (function create pure-child (visibility public) (parameter create n (type I64))
    (returns I64) (effect pure) (body (local n)))
  (function create parent (visibility public) {captures} (returns {result_type})
    (effect (task (requirement authority::clock) (requirement authority::unused)))
    (body (sequence (unit) {selected_source})))))
declarations.end"#,
            repository.view_current().unwrap().revision(),
            standard.package,
            standard.semantic_revision,
            standard.package_revision,
        );
        let request = crate::platform::control::decode_compact_change(
            "parallel-extraction-fixture",
            literal.as_bytes(),
        )
        .unwrap();
        let prepared = repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        repository.publish(&prepared.publication).unwrap();
        let source = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        let requirements: Vec<RequirementOperand> = source
            .owners
            .iter()
            .filter_map(|(key, owner)| {
                let (OwnerKey::Requirement(id), OwnerRecord::Requirement(record)) = (key, owner)
                else {
                    return None;
                };
                (uses_clock && record.name.as_str() == "clock").then_some(
                    RequirementOperand::Concrete(RequirementReference {
                        package: source.root.package_id,
                        requirement: *id,
                    }),
                )
            })
            .collect();
        assert_eq!(requirements.len(), usize::from(uses_clock));
        let (function, selected) = source
            .owners
            .iter()
            .find_map(|(key, owner)| {
                let (OwnerKey::Declaration(function), OwnerRecord::Declaration(d)) = (key, owner)
                else {
                    return None;
                };
                if d.name.as_str() != "parent" {
                    return None;
                }
                let DeclarationPayload::Function(f) = &d.payload else {
                    return None;
                };
                let OwnerRecord::Expression(e) = &source.owners[&OwnerKey::Expression(f.body)]
                else {
                    return None;
                };
                let ExpressionOperation::Sequence { items } = &e.operation else {
                    return None;
                };
                Some((*function, items[1]))
            })
            .unwrap();
        let independent = crate::platform::contributor::function_extraction_oracle(
            &project,
            &function.to_string(),
            &selected.to_string(),
        )
        .unwrap();
        assert_eq!(independent.effect, expected_kind);
        assert_eq!(
            independent.requirements,
            requirements
                .iter()
                .map(|r| format!("{}/{}", r.package(), r.owner()))
                .collect::<Vec<_>>()
        );
        let literal = format!(
            "request base={}\nextract.function as=$helper function={function} expression={selected} name=extracted\n",
            repository.view_current().unwrap().revision()
        );
        let request = crate::platform::control::decode_compact_change(
            "parallel-extraction",
            literal.as_bytes(),
        )
        .unwrap();
        let prepared = repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        let extraction = prepared.logical_plan.extraction.as_ref().unwrap();
        let helper = extraction.helper;
        match &extraction.effect {
            FunctionEffect::Pure => assert_eq!(expected_kind, "pure"),
            FunctionEffect::Task {
                requirements: actual_requirements,
                effect_parameters,
            } => {
                assert_eq!(expected_kind, "task");
                assert_eq!(*actual_requirements, requirements);
                assert!(effect_parameters.is_empty());
            }
        }
        repository.publish(&prepared.publication).unwrap();
        let after = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        let OwnerRecord::Declaration(helper_record) = &after.owners[&OwnerKey::Declaration(helper)]
        else {
            panic!("helper")
        };
        let DeclarationPayload::Function(helper_function) = &helper_record.payload else {
            panic!("helper")
        };
        assert_eq!(
            helper_function.body, selected,
            "extraction must preserve the accepted subtree identities"
        );
        assert_eq!(helper_function.effect, extraction.effect);
        assert!(validate_full(&after).is_ok());
    }
}
