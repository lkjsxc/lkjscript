use super::*;

const EFFECT_LIBRARY: &str = include_str!("../fixtures/effect-resources-library.lkjc");
const EFFECT_CONSUMER: &str = include_str!("../fixtures/effect-resources-consumer.lkjc");

fn stage(source: &str) -> Packages {
    let packages = Packages::stage(EFFECT_LIBRARY);
    packages.apply(source);
    packages
}

fn descriptor(public: &Native, target: &str) -> PathBuf {
    let deployment = public.root.path().join("effects.deployment.json");
    write_deployment(&deployment, target);
    let mut value: Value = serde_json::from_slice(&std::fs::read(&deployment).unwrap()).unwrap();
    let mut audit = value["grants"][0].clone();
    audit["requirement"] = serde_json::json!("audit");
    audit["adapter"]["root"] = serde_json::json!("audit");
    value["grants"].as_array_mut().unwrap().push(audit);
    std::fs::write(&deployment, serde_json::to_vec(&value).unwrap()).unwrap();
    deployment
}

fn initialize_audit(public: &Native) -> PathBuf {
    let audit = public.root.path().join("audit");
    compact_success_at(
        &public.executable,
        public.root.path(),
        &["data", "initialize", "--root", path(&audit)],
    );
    audit
}

#[test]
fn native_effect_generic_resources_compose_with_a_repeated_borrow_suffix() {
    let library = EFFECT_LIBRARY.replacen(
        "(returns T) (effect (task (requirement queue::jobs) (parameter E)))",
        "(parameter create other (type (resource std::DurableQueue)) (use borrow) (requirement queue::jobs))\n      (returns T) (effect (task (requirement queue::jobs) (parameter E)))", 1)
        .replacen("(body (call repeat-read (types T) (effects (row (parameter E)))\n        (i64 2) (local decode) (local lease)))",
            "(body (sequence (capability-call queue::jobs std::DurableQueue::lease-info (local other)) (call repeat-read (types T) (effects (row (parameter E)))\n        (i64 2) (local decode) (local lease))))", 1);
    assert!(library.contains("(local other)"));
    let source = EFFECT_CONSUMER
        .replace("(call lib::read-lease (types U) (effects (row (parameter E)))\n        (local decode) (local lease))", "(call lib::read-lease (types U) (effects (row (parameter E)))\n        (local decode) (local lease) (local lease))")
        .replace("(call lib::read-lease (types T) (effects (row (requirement app::audit)))\n                (local decode) (local lease))", "(call lib::read-lease (types T) (effects (row (requirement app::audit)))\n                (local decode) (local lease) (local lease))");
    assert_eq!(source.matches("(local lease) (local lease)").count(), 2);
    assert_effect_callback_execution(&library, &source);
}

#[test]
fn native_effect_generic_resources_reject_mismatch_and_escape_without_publication() {
    let packages = Packages::stage(EFFECT_LIBRARY);
    let public = &packages.consumer;
    let base = public.revision();
    let inventory = content_inventory(&public.project);
    for (name, old, new, code) in [
        (
            "missing-effect",
            "(effects (row (parameter E)))",
            "",
            "kernel_effect_argument_count",
        ),
        (
            "wrong-effect",
            "(effects (row (parameter E)))",
            "(effects (row))",
            "kernel_type_argument",
        ),
        (
            "implicit-resource-authority",
            "(returns U) (effect (task (requirement lib::queue::jobs) (parameter E)))",
            "(returns U) (effect (task (parameter E)))",
            "kernel_affine_function_resource_effect",
        ),
        (
            "callback-authority-is-not-resource-authority",
            "(use borrow) (requirement lib::queue::jobs)",
            "(use borrow) (requirement app::audit)",
            "kernel_affine_function_resource_effect",
        ),
        (
            "borrow-consumed",
            "(body (call lib::read-lease (types U) (effects (row (parameter E)))\n        (local decode) (local lease))))",
            "(body (call lib::finish (types U) (effects (row (parameter E))) (local decode) (call std::data-encode (types Unit) (unit)) (local lease))))",
            "kernel_affine_borrow_consumed",
        ),
        (
            "function-escape",
            "(binding first (call relay",
            "(binding escaped (function-value lib::read-lease (types T) (effects (row (requirement app::audit))))) (binding first (call relay",
            "kernel_affine_resource_function_value",
        ),
    ] {
        assert!(EFFECT_CONSUMER.contains(old), "literal anchor {name}");
        let source = EFFECT_CONSUMER.replacen(old, new, 1);
        let input = public.input(&format!("{name}.lkjc"), &packages.request(&source));
        let result = public.plan(&input, false);
        assert!(
            result
                .iter()
                .any(|record| record.operation == "diagnostic"
                    && compact_field(record, "code") == code),
            "{name}: {result:?}"
        );
        assert_eq!(public.revision(), base, "{name}");
        assert_eq!(content_inventory(&public.project), inventory, "{name}");
    }
    packages.apply(EFFECT_CONSUMER);
}

#[test]
fn native_effect_generic_resources_require_callback_grants_before_any_effect() {
    let packages = stage(EFFECT_CONSUMER);
    let data = packages.detach();
    let audit = initialize_audit(&packages.consumer);
    let before = (content_inventory(&data), content_inventory(&audit));
    let deployment = packages.consumer.root.path().join("missing-effect.json");
    write_deployment(&deployment, "numbers");
    let result = packages
        .consumer
        .cli(&["run", "--deployment", path(&deployment)], false);
    assert!(
        result.iter().any(|record| record.operation == "diagnostic"
            && compact_field(record, "code") == "deployment_grant_missing"),
        "{result:?}"
    );
    assert_eq!(
        (content_inventory(&data), content_inventory(&audit)),
        before
    );
}

fn empty_callbacks() -> String {
    let mut source = EFFECT_CONSUMER
        .replace("(row (requirement app::audit))", "(row)")
        .replace(
            "(effect (task (requirement app::audit)))",
            "(effect (task))",
        );
    for job in ["numbers", "text"] {
        let effects = format!(
            "        (capability-call app::audit std::DurableQueue::initialize)\n        (capability-call app::audit std::DurableQueue::enqueue\n          (text \"{job}\") (text \"{job}\") (local payload) (i64 0) (i64 0))\n"
        );
        assert!(source.contains(&effects));
        source = source.replace(&effects, "");
    }
    source
}

#[test]
fn native_effect_generic_resources_accept_empty_task_callbacks_without_pure_coercion() {
    let source = empty_callbacks();
    let packages = stage(&source);
    let pure = source.replace("(effect (task))", "(effect pure)");
    // A separate unmodified consumer proves pure/task distinction at application.
    let rejected_packages = Packages::stage(EFFECT_LIBRARY);
    let base = rejected_packages.consumer.revision();
    let inventory = content_inventory(&rejected_packages.consumer.project);
    let input = rejected_packages
        .consumer
        .input("pure-callback.lkjc", &rejected_packages.request(&pure));
    let result = rejected_packages.consumer.plan(&input, false);
    assert!(
        result.iter().any(|record| record.operation == "diagnostic"
            && compact_field(record, "code") == "kernel_type_argument"),
        "{result:?}"
    );
    assert_eq!(rejected_packages.consumer.revision(), base);
    assert_eq!(
        content_inventory(&rejected_packages.consumer.project),
        inventory
    );
    let data = packages.detach();
    initialize_audit(&packages.consumer);
    let deployment = descriptor(&packages.consumer, "numbers");
    let result = packages
        .consumer
        .cli(&["run", "--deployment", path(&deployment)], true);
    assert_eq!(
        serde_json::from_str::<Value>(compact_field(compact_record(&result, "execution"), "value"))
            .unwrap(),
        serde_json::json!([7, 42, -3])
    );
    assert_completed_job(&read_job(&data, "numbers"), "numbers");
}

#[test]
fn native_effect_generic_callback_failure_distinguishes_borrow_from_completed_transfer() {
    let decoded = "(call std::data-decode-or (types (list I64)) (local payload) (list I64))";
    let failed = EFFECT_CONSUMER.replace(decoded, "(list I64 (call std::divide (i64 1) (i64 0)))");
    assert_ne!(failed, EFFECT_CONSUMER);
    let borrowed = "(let\n              (binding first (call relay (types T) (effects (row (requirement app::audit)))\n                (local decode) (local lease)))\n              (binding second (call lib::read-lease (types T) (effects (row (requirement app::audit)))\n                (local decode) (local lease)))\n              (in (call finish-here (types T) (effects (row (requirement app::audit)))\n                (local decode) (local payload) (local lease))))";
    let consumed = "(call finish-here (types T) (effects (row (requirement app::audit))) (local decode) (local payload) (local lease))";
    assert!(failed.contains(borrowed));
    for after_consume in [false, true] {
        let source = if after_consume {
            failed.replace(borrowed, consumed)
        } else {
            failed.clone()
        };
        let packages = stage(&source);
        let data = packages.detach();
        let audit = initialize_audit(&packages.consumer);
        let deployment = descriptor(&packages.consumer, "numbers");
        let result = packages
            .consumer
            .cli(&["run", "--deployment", path(&deployment)], false);
        assert!(
            result.iter().any(|record| record.operation == "diagnostic"
                && compact_field(record, "code") == "normalized_integer_division"),
            "{result:?}"
        );
        let original = read_job(&data, "numbers");
        if after_consume {
            assert_completed_job(&original, "numbers");
        } else {
            super::super::failure::assert_leased_job(&original, b"effect-worker");
        }
        let audit_job = read_job(&audit, "numbers");
        let mut cursor = job_payload(&audit_job);
        assert_eq!(take(&mut cursor, 8), b"LKJQJOB1");
        assert_eq!(blob(&mut cursor), b"numbers");
        assert_eq!(blob(&mut cursor), b"numbers");
        let payload = blob(&mut cursor);
        assert!(!payload.is_empty());
        assert_eq!(take(&mut cursor, 1), [0]);
        let mut main = job_payload(&original);
        take(&mut main, 8);
        blob(&mut main);
        blob(&mut main);
        assert_eq!(blob(&mut main), payload);
        let deployment = descriptor(&packages.consumer, "text");
        let result = packages
            .consumer
            .cli(&["run", "--deployment", path(&deployment)], true);
        assert_eq!(
            serde_json::from_str::<Value>(compact_field(
                compact_record(&result, "execution"),
                "value"
            ))
            .unwrap(),
            serde_json::json!("日本語 + generic")
        );
        assert_completed_job(&read_job(&data, "text"), "text");
        assert_eq!(read_job(&data, "numbers"), original);
    }
}
