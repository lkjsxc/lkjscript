//! Same-task owned memory and exact capability resources retain separate custody.
use super::*;

fn program(generic: bool) -> String {
    let parameter = if generic { "T" } else { "ByteBuffer" };
    let declaration = if generic {
        "(type-parameter create T (constraint owned))"
    } else {
        ""
    };
    let forwarding_types = if generic { "(types T)" } else { "" };
    let call_types = if generic { "(types ByteBuffer)" } else { "" };
    let helpers = format!(
        r#"(module create generic-queue (as $module)
    (external create empty (visibility private) (implementation core.buffer.empty) (returns ByteBuffer))
    (external create discard (visibility private) (implementation core.buffer.discard)
      (parameter create owner (type ByteBuffer) (use consume)) (returns Unit))
    (function create allocate (visibility private) (effect (task)) (returns ByteBuffer)
      (body (call empty)))
    (function create owned-finish (visibility private)
      {declaration}
      (parameter create result (type Bytes))
      (parameter create owner (type {parameter}) (use consume))
      (parameter create lease (type (resource std::DurableQueue))
        (use consume) (requirement queue::jobs))
      (returns {parameter}) (effect (task (requirement queue::jobs)))
      (body (sequence
        (capability-call queue::jobs std::DurableQueue::complete
          (local lease) (i64 0) (local result))
        (local owner))))
    (function create owned-forward (visibility private)
      {declaration}
      (parameter create result (type Bytes))
      (parameter create owner (type {parameter}) (use consume))
      (parameter create lease (type (resource std::DurableQueue))
        (use consume) (requirement queue::jobs))
      (returns {parameter}) (effect (task (requirement queue::jobs)))
      (body (call owned-finish {forwarding_types} (local result) (local owner) (local lease))))"#
    );
    let old = "(body (sequence\n        (capability-call queue::jobs std::DurableQueue::complete\n          (local lease) (i64 0) (local result))\n        (local value))))";
    let body = format!(
        r#"(body (let
        (binding buffer (type ByteBuffer) (call allocate))
        (binding returned (type ByteBuffer)
          (call owned-forward {call_types} (local result) (local buffer) (local lease)))
        (in (sequence (call discard (local returned)) (local value))))))"#
    );
    assert!(PROGRAM.contains(old));
    PROGRAM
        .replacen("(module create generic-queue (as $module)", &helpers, 1)
        .replacen(old, &body, 1)
}

#[test]
fn native_task_owned_order_and_effect_rows_reject_without_publication() {
    let valid = program(false);
    let original = "(parameter create owner (type ByteBuffer) (use consume))\n      (parameter create lease (type (resource std::DurableQueue))\n        (use consume) (requirement queue::jobs))";
    let reordered = "(parameter create lease (type (resource std::DurableQueue))\n        (use consume) (requirement queue::jobs))\n      (parameter create owner (type ByteBuffer) (use consume))";
    assert!(valid.contains(original));
    let wrong_order = valid.replacen(original, reordered, 1).replacen(
        "(local result) (local owner) (local lease)",
        "(local result) (local lease) (local owner)",
        1,
    );
    let effect = "(returns ByteBuffer) (effect (task (requirement queue::jobs)))";
    assert!(valid.contains(effect));
    let missing_effect = valid.replacen(effect, "(returns ByteBuffer) (effect (task))", 1);
    for (name, invalid) in [
        ("resource-before-memory", wrong_order),
        ("missing-effect", missing_effect),
    ] {
        let n = Native::new();
        let before = n.revision();
        let request = n.input(
            &format!("{name}.lkjc"),
            &format!("request base={before}\n{invalid}"),
        );
        let records = n.plan(&request, false);
        assert!(
            records.iter().any(|record| record.operation == "diagnostic"
                && compact_field(record, "class") == "semantic"),
            "{name}: {records:?}"
        );
        assert_eq!(n.revision(), before);
    }
}

#[test]
fn native_task_owned_concrete_memory_composes_with_exact_resources() {
    check_program(&program(false));
}

#[test]
fn native_task_owned_generic_memory_composes_with_exact_resources() {
    check_program(&program(true));
}
