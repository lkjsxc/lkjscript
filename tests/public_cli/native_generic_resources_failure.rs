//! A failed borrowed decoder cannot turn invocation cleanup into durable completion.
use super::*;

#[test]
fn native_type_generic_resources_preserve_failed_lease_without_blocking_other_jobs() {
    let public = Native::template("command");
    let original =
        "(body (call std::data-decode-or (types (list I64)) (local payload) (list I64))))";
    assert_eq!(PROGRAM.matches(original).count(), 1);
    let source = PROGRAM.replacen(
        original,
        "(body (list I64 (call std::divide (i64 1) (i64 0)))))",
        1,
    );
    let input = public.input(
        "failing-decoder.lkjc",
        &format!("request base={}\n{source}", public.revision()),
    );
    public.apply(&input, &public.plan(&input, true), true);
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
    write_deployment(&deployment, "numbers");
    let rejected = public.cli(&["run", "--deployment", path(&deployment)], false);
    assert!(
        rejected
            .iter()
            .any(|record| record.operation == "diagnostic"
                && compact_field(record, "code") == "normalized_integer_division")
    );
    let failed_job = read_job(&data, "numbers");
    assert_leased_job(&failed_job);

    // The earlier claim remains committed; it is neither a completion nor a rollback.
    // A separate invocation can nevertheless claim and complete its own ready job.
    write_deployment(&deployment, "text");
    let output = public.cli(&["run", "--deployment", path(&deployment)], true);
    let actual: Value =
        serde_json::from_str(compact_field(compact_record(&output, "execution"), "value")).unwrap();
    assert_eq!(actual, serde_json::json!("日本語 + generic"));
    assert_completed_job(&read_job(&data, "text"), "text");
    assert_eq!(read_job(&data, "numbers"), failed_job);
}

fn assert_leased_job(bytes: &[u8]) {
    let mut cursor = job_payload(bytes);
    assert_eq!(take(&mut cursor, 8), b"LKJQJOB1");
    assert_eq!(blob(&mut cursor), b"numbers");
    assert_eq!(blob(&mut cursor), b"numbers");
    assert!(!blob(&mut cursor).is_empty());
    assert_eq!(take(&mut cursor, 1), [1]); // leased, not completed
    assert_eq!(take(&mut cursor, 16), [0; 16]); // availability and creation
    assert_eq!(take(&mut cursor, 4), 1_u32.to_be_bytes());
    assert_eq!(take(&mut cursor, 1), [1]); // attempt present
    assert_eq!(blob(&mut cursor), b"numbers:1");
    assert_eq!(take(&mut cursor, 1), [1]); // worker present
    assert_eq!(blob(&mut cursor), b"generic-worker");
    assert_eq!(take(&mut cursor, 1), [1]); // lease present
    assert_eq!(take(&mut cursor, 8), 1000_i64.to_be_bytes());
    assert_eq!(take(&mut cursor, 2), [0, 0]); // no completion result or queue error
    assert!(cursor.is_empty());
}
