use super::*;

const SUFFIX: &str = include_str!("../fixtures/resource-suffix.lkjc");

#[test]
fn native_resource_suffix_keeps_exact_authority_through_recursion_and_detached_execution() {
    check_suffix(SUFFIX);
}

#[test]
fn native_resource_suffix_mixes_borrow_and_consume_in_both_orders() {
    for (side, requirement, owner, payload) in [
        ("left", "jobs", "first", "左 + first"),
        ("right", "other", "second", "右 + second"),
    ] {
        let source = replace_once(
            SUFFIX,
            &format!("(use consume) (requirement queue::{requirement}))"),
            &format!("(use borrow) (requirement queue::{requirement}))"),
        );
        let source = replace_once(
            &source,
            &format!(
                "(capability-call queue::{requirement} std::DurableQueue::complete\n              (local {side}) (i64 0) (local {side}-data))"
            ),
            &format!(
                "(capability-call queue::{requirement} std::DurableQueue::lease-info (local {side}))"
            ),
        );
        let call = "(call finish (types (list I64)) (i64 8)\n                    (local value) (local first) (local second))";
        let source = replace_once(
            &source,
            call,
            &format!(
                "(sequence {call}\n (capability-call queue::{requirement} std::DurableQueue::complete (local {owner}) (i64 0) (call std::data-encode (types Text) (text \"{payload}\"))) (local value))"
            ),
        );
        check_suffix(&source);
    }
}

#[test]
fn native_resource_suffix_non_tail_consume_preserves_both_owners() {
    let call = "(call finish (types V) (call std::subtract (local remaining) (i64 1))\n          (local value) (local left) (local right))";
    let source = replace_once(
        SUFFIX,
        call,
        &format!("(let (binding done {call}) (in (local value)))"),
    );
    check_suffix(&source);
}

#[test]
fn native_resource_suffix_rejects_aliases_and_invalid_later_parameters_without_publication() {
    let public = Native::template("command");
    let base = public.revision();
    let inventory = content_inventory(&public.project);
    let call = "(call finish (types (list I64)) (i64 8)\n                    (local value) (local first) (local second))";
    let alias = "(call finish (types (list I64)) (i64 8)\n                    (local value) (local first) (local first))";
    let mut cases = Vec::new();
    for borrowed in [None, Some(("left", "jobs")), Some(("right", "other"))] {
        let mut source = SUFFIX.to_owned();
        if let Some((side, requirement)) = borrowed {
            source = replace_once(
                &source,
                &format!("(use consume) (requirement queue::{requirement}))"),
                &format!("(use borrow) (requirement queue::{requirement}))"),
            );
            source = replace_once(
                &source,
                &format!(
                    "(capability-call queue::{requirement} std::DurableQueue::complete\n              (local {side}) (i64 0) (local {side}-data))"
                ),
                &format!(
                    "(capability-call queue::{requirement} std::DurableQueue::lease-info (local {side}))"
                ),
            );
        }
        cases.push((
            format!("alias-{borrowed:?}"),
            replace_once(&source, call, alias),
            "kernel_affine_resource_call_alias",
        ));
    }
    for (name, old, new, expected) in [
        (
            "wrong-second-authority",
            "(local first) (local second) (local first)",
            "(local first) (local first) (local first)",
            "kernel_affine_foreign_requirement",
        ),
        (
            "ordinary-after-resource",
            "(parameter create again (type (resource std::DurableQueue))",
            "(parameter create gap (type I64)) (parameter create again (type (resource std::DurableQueue))",
            "kernel_affine_function_resource_order",
        ),
        (
            "later-resource-unrestricted",
            "(use borrow) (requirement queue::other))",
            "(use unrestricted) (requirement queue::other))",
            "kernel_affine_function_resource_use",
        ),
        (
            "later-resource-unbound",
            "(use borrow) (requirement queue::other))",
            "(use borrow))",
            "kernel_affine_function_resource_requirement",
        ),
        (
            "later-borrow-consumed",
            "(capability-call queue::jobs std::DurableQueue::lease-info (local again))",
            "(capability-call queue::jobs std::DurableQueue::complete (local again) (i64 0) (call std::data-encode (types Text) (text \"invalid\")))",
            "kernel_affine_borrow_consumed",
        ),
        (
            "indirect",
            "(binding value (call observe",
            "(binding escaped (function-value observe (types (list I64)))) (binding value (call observe",
            "kernel_affine_resource_function_value",
        ),
        (
            "nonlocal-resource",
            "(local first) (local second) (local first)",
            "(local first) (local second) (sequence (unit) (local first))",
            "kernel_affine_resource_argument",
        ),
    ] {
        cases.push((name.into(), replace_once(SUFFIX, old, new), expected));
    }
    cases.push(("use-after-consume".into(), replace_once(SUFFIX, call,
        &format!("(sequence {call} (capability-call queue::other std::DurableQueue::lease-info (local second)) (local value))")),
        "kernel_affine_use_after_consume"));
    for (index, (name, source, expected)) in cases.into_iter().enumerate() {
        let input = public.input(
            &format!("rejected-{index}.lkjc"),
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

fn replace_once(source: &str, old: &str, new: &str) -> String {
    assert_eq!(
        source.matches(old).count(),
        1,
        "literal fixture anchor: {old}"
    );
    source.replacen(old, new, 1)
}

fn check_suffix(source: &str) {
    let public = Native::template("command");
    let input = public.input(
        "suffix.lkjc",
        &format!("request base={}\n{source}", public.revision()),
    );
    let applied = public.apply(&input, &public.plan(&input, true), true);
    public.cli(&["check"], true);
    let draft = public.root.path().join("suffix-draft.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            &identity(&applied, "$module"),
            "--output",
            path(&draft),
        ],
        true,
    );
    assert_eq!(
        compact_field(
            compact_record(&public.plan(&draft, true), "result"),
            "outcome"
        ),
        "unchanged"
    );
    let artifact = public.root.path().join("suffix.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let data = public.root.path().join("queue");
    compact_success_at(
        &public.executable,
        public.root.path(),
        &["data", "initialize", "--root", path(&data)],
    );
    std::fs::rename(&public.project, public.root.path().join("retained-project")).unwrap();
    let deployment = public.root.path().join("suffix.deployment.json");
    let grants = [("jobs", "left"), ("other", "right")].map(|(requirement, job)| serde_json::json!({
        "requirement":requirement, "sharing_domain":format!("suffix-{job}"), "authority_revision":"b3".repeat(32),
        "adapter":{"kind":"durable_queue_data", "root":"queue", "namespace":format!("suffix-{job}"),
            "data_limits":DataLimits::default(), "limits":QueueLimits::default()}
    }));
    let descriptor = serde_json::json!({
        "artifact":"suffix.lkja", "target":"pair", "listen":null, "http":null, "session":null, "worker":null,
        "streams":lkjscript::platform::stream::StreamLimits::default(), "configuration":{}, "secrets":[], "grants":grants
    });
    std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    for expected in [serde_json::json!([7, 42, -3]), serde_json::json!([])] {
        let output = public.cli(&["run", "--deployment", path(&deployment)], true);
        let actual: Value =
            serde_json::from_str(compact_field(compact_record(&output, "execution"), "value"))
                .unwrap();
        assert_eq!(actual, expected);
        let cleanup: Value = serde_json::from_str(compact_field(
            compact_record(&output, "execution"),
            "cleanup",
        ))
        .unwrap();
        assert_eq!(cleanup["remaining_tasks"], 0);
        assert_eq!(cleanup["cleanup_failures"], serde_json::json!([]));
    }
    for job in ["left", "right"] {
        let store = DataStore::open(&data, format!("suffix-{job}"), DataLimits::default()).unwrap();
        let key = DataKey::new(vec![DataKeyPart::Text(job.into())], store.limits()).unwrap();
        let value = store
            .begin()
            .unwrap()
            .get("__queue.jobs", &key)
            .unwrap()
            .unwrap()
            .value;
        assert_completed_job(&value, job);
    }
}
