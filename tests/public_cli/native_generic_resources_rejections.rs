use super::*;

#[test]
fn native_type_generic_resources_reject_escaping_or_rebound_authority_without_publication() {
    let public = Native::template("command");
    let base = public.revision();
    let inventory = content_inventory(&public.project);
    for (name, old, replacement, expected) in [
        (
            "missing-effect-argument",
            "(type-parameter create T)",
            "(type-parameter create T) (effect-parameter create E)",
            "kernel_effect_argument_count",
        ),
        (
            "requirement-parameter",
            "(type-parameter create T)",
            "(type-parameter create T) (requirement-parameter create R (interface std::DurableQueue) (operations std::DurableQueue::lease-info))",
            "kernel_affine_function_resource_generic",
        ),
        (
            "consume-borrow",
            "(body (call read-lease (types U) (local decode) (local lease)))",
            "(body (sequence (capability-call queue::jobs std::DurableQueue::complete (local lease) (i64 0) (call std::data-encode (types Text) (text \"done\"))) (call read-lease (types U) (local decode) (local lease))))",
            "kernel_affine_borrow_consumed",
        ),
        (
            "use-after-consume",
            "(in (call finish (types T) (local second) (local payload) (local lease)))",
            "(in (sequence (call finish (types T) (local second) (local payload) (local lease)) (call read-lease (types T) (local decode) (local lease))))",
            "kernel_affine_use_after_consume",
        ),
        (
            "ordinary-type-resource",
            "(binding second (call read-lease (types T) (local decode) (local lease)))",
            "(binding copied (call finish (types (resource std::DurableQueue)) (local lease) (local payload) (local lease))) (binding second (call read-lease (types T) (local decode) (local lease)))",
            "kernel_affine_resource_copy",
        ),
        (
            "indirect",
            "(binding second (call read-lease (types T) (local decode) (local lease)))",
            "(binding escaped (function-value read-lease (types T))) (binding second (call read-lease (types T) (local decode) (local lease)))",
            "kernel_affine_resource_function_value",
        ),
        (
            "recursive-borrow-consumption",
            "(body (call read-lease (types U) (local decode) (local lease)))",
            "(body (sequence (call relay (types U) (local decode) (local lease)) (capability-call queue::jobs std::DurableQueue::complete (local lease) (i64 0) (call std::data-encode (types Text) (text \"done\"))) (call read-lease (types U) (local decode) (local lease))))",
            "kernel_affine_borrow_consumed",
        ),
    ] {
        assert!(PROGRAM.contains(old), "literal edit anchor: {name}");
        let source = PROGRAM.replacen(old, replacement, 1);
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
