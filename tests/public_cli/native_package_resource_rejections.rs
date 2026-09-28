use super::*;

#[test]
fn native_package_resources_never_share_one_named_grant_between_distinct_requirements() {
    let source = CONSUMER.replace("(component create app (visibility private)",
        "(component create app (visibility private) (requirement create jobs (interface std::DurableQueue) (operations std::DurableQueue::initialize) (limits (maximum_calls 64 calls)))");
    let packages = Packages::stage(LIBRARY);
    packages.apply(&source);
    let data = packages.detach();
    let inventory = content_inventory(&data);
    let deployment = packages.consumer.root.path().join("collision.json");
    // Static admission and preparation both consume each named grant once.
    // Padding the count cannot grant both exact owners or defer rejection.
    for (mode, code) in [
        ("single", "deployment_grant_missing"),
        ("duplicate", "deployment_grant_duplicate"),
        ("padded", "deployment_grant_missing"),
    ] {
        write_deployment(&deployment, "numbers");
        if mode != "single" {
            let mut descriptor: Value =
                serde_json::from_slice(&std::fs::read(&deployment).unwrap()).unwrap();
            let mut extra = descriptor["grants"][0].clone();
            if mode == "padded" {
                extra["requirement"] = serde_json::json!("unused-jobs");
            }
            descriptor["grants"].as_array_mut().unwrap().push(extra);
            std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
        }
        let rejected = packages
            .consumer
            .cli(&["run", "--deployment", path(&deployment)], false);
        assert!(
            rejected
                .iter()
                .any(|record| record.operation == "diagnostic"
                    && compact_field(record, "code") == code),
            "{code}: {rejected:?}"
        );
        assert_eq!(content_inventory(&data), inventory);
    }
}

#[test]
fn native_package_resources_reject_rebinding_escalation_and_hidden_calls_before_publication() {
    let packages = Packages::stage(LIBRARY);
    let consumer = &packages.consumer;
    let base = consumer.revision();
    let inventory = content_inventory(&consumer.project);
    let reader = "(body (call lib::read-lease (types U)\n        (local decode) (local lease))))";
    let handoff = "(in (call finish-here (types T) (local second) (local payload) (local lease)))";
    let first = "(binding first (call relay (types T) (local decode) (local lease)))";
    let mut cases = Vec::new();
    for (name, old, new, code) in [
        (
            "borrow-consumed",
            reader,
            "(body (call lib::finish (types U) (call lib::read-lease (types U) (local decode) (local lease)) (field (capability-call lib::queue::jobs std::DurableQueue::lease-info (local lease)) std::QueueLeaseInfo::payload) (local lease))))",
            "kernel_affine_borrow_consumed",
        ),
        (
            "use-after-transfer",
            handoff,
            "(in (sequence (call finish-here (types T) (local second) (local payload) (local lease)) (call lib::read-lease (types T) (local decode) (local lease))))",
            "kernel_affine_use_after_consume",
        ),
        (
            "indirect",
            first,
            "(binding escaped (function-value lib::read-lease (types T))) (binding first (call relay (types T) (local decode) (local lease)))",
            "kernel_affine_resource_function_value",
        ),
        (
            "private-helper",
            reader,
            "(body (call lib::hidden (local lease))))",
            "change_reference_not_exposed",
        ),
        (
            "package-helper",
            reader,
            "(body (call lib::package-hidden (local lease))))",
            "change_reference_not_exposed",
        ),
        (
            "requirement-generic",
            "(type-parameter create U)",
            "(type-parameter create U) (requirement-parameter create R (interface std::DurableQueue) (operations std::DurableQueue::lease-info))",
            "kernel_affine_function_resource_generic",
        ),
        (
            "missing-port-effect",
            "(row (requirement lib::queue::jobs))",
            "(row)",
            "kernel_type_task_requirement",
        ),
    ] {
        assert!(CONSUMER.contains(old), "literal edit anchor: {name}");
        cases.push((name, CONSUMER.replacen(old, new, 1), code));
    }
    let local = CONSUMER
        .replace("(task (requirement lib::queue::jobs))",
            "(task (requirement lib::queue::jobs) (requirement app::jobs))")
        .replace("(row (requirement lib::queue::jobs))",
            "(row (requirement lib::queue::jobs) (requirement app::jobs))")
        .replace("(component create app (visibility private)",
            "(component create app (visibility private) (requirement create jobs (interface std::DurableQueue) (operations std::DurableQueue::initialize std::DurableQueue::enqueue std::DurableQueue::claim std::DurableQueue::lease-info std::DurableQueue::complete) (limits (maximum_calls 64 calls)))");
    cases.push((
        "same-name-different-owner",
        local.replacen(
            "(capability-call lib::queue::jobs std::DurableQueue::claim",
            "(capability-call app::jobs std::DurableQueue::claim",
            1,
        ),
        "kernel_affine_foreign_requirement",
    ));
    cases.push((
        "rebound-parameter",
        local.replacen(
            "(use borrow) (requirement lib::queue::jobs)",
            "(use borrow) (requirement app::jobs)",
            1,
        ),
        "kernel_affine_foreign_requirement",
    ));
    for (name, source, code) in cases {
        let input = consumer.input(&format!("{name}.lkjc"), &packages.request(&source));
        let rejected = consumer.plan(&input, false);
        assert!(
            rejected
                .iter()
                .any(|record| record.operation == "diagnostic"
                    && compact_field(record, "code") == code),
            "{name}: {rejected:?}"
        );
        assert_eq!(consumer.revision(), base, "{name}");
        assert_eq!(content_inventory(&consumer.project), inventory, "{name}");
    }
    packages.apply(CONSUMER);
}

#[test]
fn native_package_resources_require_exact_explicit_deployment_grants_before_effects() {
    let packages = Packages::stage(LIBRARY);
    packages.apply(CONSUMER);
    let data = packages.detach();
    let inventory = content_inventory(&data);
    let deployment = packages
        .consumer
        .root
        .path()
        .join("generic.deployment.json");
    for (name, code) in [
        ("missing", "deployment_grant_missing"),
        ("unused", "deployment_grant_foreign"),
    ] {
        write_deployment(&deployment, "numbers");
        let mut descriptor: Value =
            serde_json::from_slice(&std::fs::read(&deployment).unwrap()).unwrap();
        if name == "missing" {
            descriptor["grants"] = serde_json::json!([]);
        } else {
            let mut extra = descriptor["grants"][0].clone();
            extra["requirement"] = serde_json::json!("unused-jobs");
            descriptor["grants"].as_array_mut().unwrap().push(extra);
        }
        std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
        let rejected = packages
            .consumer
            .cli(&["run", "--deployment", path(&deployment)], false);
        assert!(
            rejected
                .iter()
                .any(|record| record.operation == "diagnostic"
                    && compact_field(record, "code") == code),
            "{name}: {rejected:?}"
        );
        assert_eq!(content_inventory(&data), inventory, "{name}");
    }
    assert_execution(&packages.consumer, &data);
}
