#![allow(clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

fn driver_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/durable-blob-publisher/driver.py")
}

fn run_driver(session: bool) -> Value {
    let mut command = Command::new("python3");
    command
        .arg(driver_path())
        .arg(env!("CARGO_BIN_EXE_lkjscript"));
    if session {
        command.arg("--runtime-session");
    }
    let output = command.output().expect("run durable-blob-publisher driver");
    assert!(
        output.status.success(),
        "durable-blob-publisher driver failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "driver stderr must stay empty");
    serde_json::from_slice(&output.stdout).expect("publisher summary JSON")
}

#[test]
fn public_blob_publisher_is_durable_typed_bounded_and_source_independent() {
    let summary = run_driver(false);
    assert_eq!(
        summary["contract_versions"],
        json!({"workspace": 10, "release": 1, "application": 4, "instance": 2})
    );
    assert_eq!(summary["source_workspace_deleted"], true);
    assert_eq!(summary["source_release_deleted"], true);
    assert_eq!(summary["proof"]["primary_revision"], 4);
    assert_eq!(summary["proof"]["secondary_revision"], 6);
    assert_eq!(summary["proof"]["blob_bytes"], 25);
    assert_eq!(summary["proof"]["already_present"], true);
    assert_eq!(summary["proof"]["unknown_outcome_reconciled"], true);
    assert_eq!(
        summary["proof"]["blob_digest"]
            .as_str()
            .expect("blob digest")
            .len(),
        64
    );
    assert!(
        summary["proof"]["history_bytes"]
            .as_u64()
            .expect("history bytes")
            > 0
    );
    assert_eq!(summary["measurements"]["engine_opens"], 1);
    assert_eq!(summary["measurements"]["authoring_rpc_calls"], 2);
    assert_eq!(summary["measurements"]["provider_tokens"], Value::Null);
}

#[test]
fn foreground_runtime_session_runs_the_same_complete_blob_workflow() {
    let summary = run_driver(true);
    assert_eq!(summary["proof"]["primary_revision"], 4);
    assert_eq!(summary["proof"]["secondary_revision"], 6);
    assert_eq!(summary["measurements"]["topology"], "foreground_session");
    assert_eq!(summary["measurements"]["processes"], 5);
    assert!(
        summary["measurements"]["runtime_rpc_calls"]
            .as_u64()
            .expect("runtime calls")
            > 30
    );
    let runtime = &summary["measurements"]["runtime_inspection"];
    assert_eq!(runtime["contract_version"], 1);
    assert_eq!(runtime["resources"]["queued_requests"], 0);
    assert_eq!(runtime["resources"]["cache_bytes"], 0);
    assert_eq!(runtime["counters"]["adapter_operations"], 8);
    assert!(
        runtime["counters"]["replayed_records"]
            .as_u64()
            .expect("replayed records")
            > 0
    );
}
