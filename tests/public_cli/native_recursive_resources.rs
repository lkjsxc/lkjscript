use super::*;

const RECURSIVE_PROGRAM: &str = include_str!("../fixtures/recursive-resources.lkjc");

#[test]
fn native_recursive_resources_preserve_generic_values_and_detached_queue_ownership() {
    check_program(RECURSIVE_PROGRAM);
}

#[test]
fn native_recursive_resources_preserve_failed_lease_without_implicit_completion() {
    failure::check_failed_decoder(RECURSIVE_PROGRAM);
}

#[test]
fn native_recursive_resources_allow_mutual_consumption_without_duplicate_completion() {
    let source = RECURSIVE_PROGRAM.replacen(
        "(binding returned (call finish (types V)",
        "(binding returned (call finish-next (types V)",
        1,
    );
    let source = source.replacen(
        "    (function create process (visibility private)",
        r#"
    (function create finish-next (visibility private)
      (type-parameter create W)
      (parameter create remaining (type I64))
      (parameter create value (type W))
      (parameter create result (type Bytes))
      (parameter create lease (type (resource std::DurableQueue))
        (use consume) (requirement queue::jobs))
      (returns W) (effect (task (requirement queue::jobs)))
      (body (call finish (types W) (local remaining) (local value) (local result) (local lease))))
    (function create process (visibility private)"#,
        1,
    );
    assert!(source.contains("(binding returned (call finish-next"));
    assert!(source.contains("(function create finish-next"));
    check_program(&source);
}

#[test]
fn native_recursive_resources_reject_post_transfer_reuse_and_borrow_escalation() {
    let public = Native::template("command");
    let base = public.revision();
    let inventory = content_inventory(&public.project);
    for (name, old, replacement, expected) in [
        (
            "post-recursive-consume-use",
            "(in (local returned))",
            "(in (sequence (capability-call queue::jobs std::DurableQueue::lease-info (local lease)) (local returned)))",
            "kernel_affine_use_after_consume",
        ),
        (
            "recursive-callable-escape",
            "(in (local returned))",
            "(in (sequence (function-value finish (types V)) (local returned)))",
            "kernel_affine_resource_function_value",
        ),
        (
            "mutual-borrow-consumption",
            "(body (call relay (types U) (local remaining) (local decode) (local lease)))",
            "(body (sequence (call relay (types U) (local remaining) (local decode) (local lease)) (capability-call queue::jobs std::DurableQueue::complete (local lease) (i64 0) (call std::data-encode (types Text) (text \"done\"))) (call read-lease (types U) (i64 0) (local decode) (local lease))))",
            "kernel_affine_borrow_consumed",
        ),
    ] {
        assert_eq!(RECURSIVE_PROGRAM.matches(old).count(), 1, "{name}");
        let source = RECURSIVE_PROGRAM.replacen(old, replacement, 1);
        let input = public.input(
            &format!("{name}.lkjc"),
            &format!("request base={base}\n{source}"),
        );
        let rejected = public.plan(&input, false);
        assert!(
            rejected
                .iter()
                .any(|record| record.operation == "diagnostic"
                    && compact_field(record, "code") == expected),
            "{name}: {rejected:?}"
        );
        assert_eq!(public.revision(), base, "{name}");
        assert_eq!(content_inventory(&public.project), inventory, "{name}");
    }
}
