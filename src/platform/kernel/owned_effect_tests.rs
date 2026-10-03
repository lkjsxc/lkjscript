//! Independently expected composition and scope failures at the meaning boundary.
use super::*;

const COMPOSITION: &str = r#"declarations.begin
(units (module create composition
  (interface create Clock (visibility public)
    (operation create tick (returns I64) (idempotency idempotent) (external-visibility none)))
  (interface create Other (visibility public)
    (operation create tick (returns I64) (idempotency idempotent) (external-visibility none)))
  (component create authority (visibility public)
    (requirement create clock (interface Clock) (operations Clock::tick) (limits (maximum_calls 4 calls)))
    (requirement create other (interface Other) (operations Other::tick) (limits (maximum_calls 4 calls)))
    (port create ready (type (function (I64) I64)) (function identity)))
  (owned-contract create Marker (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_23000000000000000000000000000001 inspect
      (parameters (I64 unrestricted)) (returns I64)))
  (function create identity (visibility public) (effect pure)
    (parameter create value (type I64)) (returns I64) (body (local value)))
  (owned-implementation create MarkerCell (visibility public)
    (contract Marker) (self OwnedI64Cell)
    (method method_23000000000000000000000000000001 identity))
  (function create transform (visibility public)
    (type-parameter create T (constraint owned))
    (effect-parameter create E)
    (requirement-parameter create R (interface Clock) (operations Clock::tick))
    (implementation-parameter implparam_23000000000000000000000000000001 ops Marker T)
    (parameter create callback (type (task-function () I64 (row (parameter E)))))
    (parameter create value (type T) (use consume))
    (returns T) (effect (task (parameter E) (requirement R)))
    (body (sequence
      (invoke (local callback))
      (capability-call R Clock::tick)
      (method-call parameter@transform@implparam_23000000000000000000000000000001
        Marker method_23000000000000000000000000000001 (i64 7))
      (local value))))
  (function create entry (visibility public)
    (parameter create callback
      (type (task-function () I64 (row (requirement authority::clock)))))
    (parameter create value (type OwnedI64Cell) (use consume))
    (returns OwnedI64Cell) (effect (task (requirement authority::clock)))
    (body (implementation-call transform (types OwnedI64Cell)
      (effects (row (requirement authority::clock))) (requirements authority::clock)
      (implementations concrete@MarkerCell) (local callback) (local value))))))
declarations.end
"#;

fn source() -> KernelSnapshot {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(COMPOSITION)
        .expect("closed methods, exact witness, callback effects and caller requirement compose")
}

fn entry(
    snapshot: &KernelSnapshot,
) -> (
    crate::platform::semantic_id::DeclarationId,
    crate::platform::semantic_id::ExpressionId,
) {
    snapshot
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Declaration(d) if d.name.as_str() == "entry" => {
                let OwnerKey::Declaration(id) = d.header.owner else {
                    return None;
                };
                let DeclarationPayload::Function(f) = &d.payload else {
                    return None;
                };
                Some((id, f.body))
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn owned_effect_application_preserves_full_operand_relations() {
    let snapshot = source();
    assert!(memory_reference::accepts(&snapshot));
    let (_, body) = entry(&snapshot);
    let record = &snapshot.owners[&OwnerKey::Expression(body)];
    let OwnerRecord::Expression(expression) = record else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        requirement_arguments,
        effect_arguments,
        ..
    } = &expression.operation
    else {
        unreachable!()
    };
    assert_eq!(requirement_arguments.len(), 1);
    assert_eq!(effect_arguments.len(), 1);
    let required = requirement_arguments[0];
    let edges = extract_owner_relations(
        snapshot.root.package_id,
        record.owner(),
        record,
        |ty| Ok(snapshot.types.get(&ty).cloned()),
        |_, _| Ok(None),
    )
    .unwrap();
    for kind in [
        RelationKind::RequirementArgument,
        RelationKind::FunctionRequirement,
    ] {
        assert!(
            edges.iter().any(|edge| edge.kind == kind
                && edge.target
                    == RelationEndpoint::Owner(ExactOwnerKey {
                        package: required.package(),
                        owner: required.owner()
                    })),
            "missing {kind:?}: {edges:?}"
        );
    }
    assert!(
        edges
            .iter()
            .any(|edge| edge.kind == RelationKind::ImplementationSelection)
    );
}

#[test]
fn owned_effect_application_rejects_missing_operands_and_foreign_scope() {
    for (source_text, expected) in [
        (
            COMPOSITION.replace("(effects (row (requirement authority::clock)))", ""),
            "kernel_effect_argument_count",
        ),
        (
            COMPOSITION.replace("(requirements authority::clock)", ""),
            "kernel_requirement_argument_count",
        ),
        (
            COMPOSITION.replace(
                "(requirements authority::clock)",
                "(requirements authority::other)",
            ),
            "kernel_requirement_argument_constraint",
        ),
        (
            COMPOSITION.replace(
                "(effects (row (requirement authority::clock)))",
                "(effects (row (parameter transform::E)))",
            ),
            "kernel_effect_parameter_scope",
        ),
    ] {
        let failure =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
                &source_text,
            )
            .unwrap_err();
        assert!(failure.contains(expected), "expected {expected}: {failure}");
    }
}

#[test]
fn owned_effect_application_does_not_extend_parallel_authority() {
    let snapshot = source();
    let (scope, body) = entry(&snapshot);
    assert_eq!(
        parallel::admit_call(&snapshot, body, Some(scope))
            .err()
            .unwrap()
            .code,
        "kernel_parallel_call"
    );
}

#[test]
fn owned_effect_application_does_not_make_methods_polymorphic() {
    let source_text = COMPOSITION.replace("(function create identity (visibility public) (effect pure)",
        "(function create identity (visibility public) (effect pure) (effect-parameter create MethodEffect)");
    let failure =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&source_text)
            .unwrap_err();
    assert!(failure.contains("kernel_owned_contract"), "{failure}");
}
