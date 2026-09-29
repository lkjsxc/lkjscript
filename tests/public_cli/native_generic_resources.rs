use super::*;
use lkjscript::platform::data::{DataKey, DataKeyPart, DataLimits, DataStore};
use lkjscript::platform::queue::QueueLimits;

const PROGRAM: &str = include_str!("../fixtures/type-generic-resources.lkjc");

#[path = "native_generic_resources_failure.rs"]
mod failure;
#[path = "native_package_resources.rs"]
mod packages;
#[path = "native_recursive_resources.rs"]
mod recursion;
#[path = "native_generic_resources_rejections.rs"]
mod rejections;
#[path = "native_resource_suffix.rs"]
mod suffix;

#[test]
fn native_type_generic_resources_survive_drafting_and_detached_queue_execution() {
    check_program(PROGRAM);
}

#[test]
fn native_resource_suffix_allows_repeated_shared_borrow() {
    let program = PROGRAM
        .replacen(
            "(returns T) (effect (task (requirement queue::jobs)))",
            "(parameter create other (type (resource std::DurableQueue))\n        (use borrow) (requirement queue::jobs))\n      (returns T) (effect (task (requirement queue::jobs)))",
            1,
        )
        .replacen(
            "(body (invoke (local decode)",
            "(body (sequence (capability-call queue::jobs std::DurableQueue::lease-info (local lease)) (invoke (local decode)",
            1,
        )
        .replacen("std::QueueLeaseInfo::payload))))", "std::QueueLeaseInfo::payload)))))", 1)
        .replacen("std::DurableQueue::lease-info (local lease))\n          std::QueueLeaseInfo::payload", "std::DurableQueue::lease-info (local other))\n          std::QueueLeaseInfo::payload", 1)
        .replace(
            "(call read-lease (types U) (local decode) (local lease))",
            "(call read-lease (types U) (local decode) (local lease) (local lease))",
        )
        .replace(
            "(call read-lease (types T) (local decode) (local lease))",
            "(call read-lease (types T) (local decode) (local lease) (local lease))",
        );
    check_program(&program);
}

fn check_program(program: &str) {
    let public = Native::template("command");
    let input = public.input(
        "generic.lkjc",
        &format!("request base={}\n{program}", public.revision()),
    );
    let applied = public.apply(&input, &public.plan(&input, true), true);
    public.cli(&["check"], true);
    let draft = public.root.path().join("generic-draft.lkjc");
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
    let authored = std::fs::read_to_string(&draft).unwrap();
    assert!(authored.contains("(type-parameter edit "));
    assert!(authored.contains("(use borrow)"));
    assert!(authored.contains("(use consume)"));
    assert_eq!(
        compact_field(
            compact_record(&public.plan(&draft, true), "result"),
            "outcome"
        ),
        "unchanged"
    );

    let artifact = public.root.path().join("generic.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let data = public.root.path().join("queue");
    compact_success_at(
        &public.executable,
        public.root.path(),
        &["data", "initialize", "--root", path(&data)],
    );
    std::fs::rename(&public.project, public.root.path().join("retained-project")).unwrap();
    let deployment = public.root.path().join("generic.deployment.json");
    for (target, expected) in [
        ("numbers", serde_json::json!([7, 42, -3])),
        ("numbers", serde_json::json!([])),
        ("text", serde_json::json!("日本語 + generic")),
        ("text", serde_json::json!("absent")),
    ] {
        write_deployment(&deployment, target);
        let output = public.cli(&["run", "--deployment", path(&deployment)], true);
        let actual: Value =
            serde_json::from_str(compact_field(compact_record(&output, "execution"), "value"))
                .unwrap();
        assert_eq!(actual, expected, "{target}");
        let cleanup: Value = serde_json::from_str(compact_field(
            compact_record(&output, "execution"),
            "cleanup",
        ))
        .unwrap();
        assert_eq!(cleanup["remaining_tasks"], 0);
        assert_eq!(cleanup["cleanup_failures"], serde_json::json!([]));
    }
    // The persisted queue is an independent oracle, not the helper's ignored Bool.
    for job in ["numbers", "text"] {
        assert_completed_job(&read_job(&data, job), job);
    }
}

fn write_deployment(path: &Path, target: &str) {
    let descriptor = serde_json::json!({
        "artifact":"generic.lkja", "target":target, "listen":null, "http":null, "session":null, "worker":null,
        "streams":lkjscript::platform::stream::StreamLimits::default(), "configuration":{}, "secrets":[],
        "grants":[{"requirement":"jobs", "sharing_domain":"generic-resources", "authority_revision":"b3".repeat(32),
            "adapter":{"kind":"durable_queue_data", "root":"queue", "namespace":"generic-resources",
                "data_limits":DataLimits::default(), "limits":QueueLimits::default()}}]
    });
    std::fs::write(path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
}

fn read_job(data: &Path, job: &str) -> Vec<u8> {
    let store = DataStore::open(data, "generic-resources", DataLimits::default()).unwrap();
    let key = DataKey::new(vec![DataKeyPart::Text(job.into())], store.limits()).unwrap();
    store
        .begin()
        .unwrap()
        .get("__queue.jobs", &key)
        .unwrap()
        .unwrap()
        .value
}

// A read-only assertion on this fixture's exact persisted state, independent of
// the product queue adapter, its job decoder, and the helper's returned value.
fn job_payload(bytes: &[u8]) -> &[u8] {
    let (payload, checksum) = bytes.split_at(bytes.len().checked_sub(32).unwrap());
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.queue.data-job.v1");
    hasher.update(&(payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    assert_eq!(hasher.finalize().as_bytes(), checksum);
    payload
}

fn assert_completed_job(bytes: &[u8], job: &str) {
    let mut cursor = job_payload(bytes);
    assert_eq!(take(&mut cursor, 8), b"LKJQJOB1");
    assert_eq!(blob(&mut cursor), job.as_bytes());
    assert_eq!(blob(&mut cursor), job.as_bytes());
    let original = blob(&mut cursor);
    assert!(!original.is_empty());
    assert_eq!(take(&mut cursor, 1), [2]); // completed
    assert_eq!(take(&mut cursor, 16), [0; 16]); // availability and creation
    assert_eq!(take(&mut cursor, 4), 1_u32.to_be_bytes());
    assert_eq!(take(&mut cursor, 3), [0; 3]); // no attempt, worker, or lease
    assert_eq!(take(&mut cursor, 1), [1]); // result is present
    assert_eq!(blob(&mut cursor), original);
    assert_eq!(take(&mut cursor, 1), [0]); // no last error
    assert!(cursor.is_empty());
}

fn take<'a>(bytes: &mut &'a [u8], length: usize) -> &'a [u8] {
    let (selected, remaining) = bytes.split_at(length);
    *bytes = remaining;
    selected
}

fn blob<'a>(bytes: &mut &'a [u8]) -> &'a [u8] {
    let length = u32::from_be_bytes(take(bytes, 4).try_into().unwrap()) as usize;
    take(bytes, length)
}
