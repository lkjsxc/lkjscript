#![allow(clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

fn driver_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/durable-controller/driver.py")
}

#[test]
fn public_controller_is_durable_exact_isolated_and_source_independent() {
    let output = Command::new("python3")
        .arg(driver_path())
        .arg(env!("CARGO_BIN_EXE_lkjscript"))
        .output()
        .expect("run durable-controller driver");
    assert!(
        output.status.success(),
        "durable-controller driver failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "driver stderr must stay empty");
    let summary: Value = serde_json::from_slice(&output.stdout).expect("controller summary JSON");
    assert_eq!(
        summary["contract_versions"],
        json!({"workspace": 10, "release": 1, "application": 3, "instance": 1})
    );
    assert_eq!(summary["source_workspace_deleted"], true);
    assert_eq!(summary["source_release_deleted"], true);
    assert_eq!(summary["proof"]["primary_revision"], 4);
    assert_eq!(summary["proof"]["history_records"], 5);
    assert!(
        summary["proof"]["history_bytes"]
            .as_u64()
            .expect("history bytes")
            > 0
    );
    assert_eq!(summary["proof"]["secondary_revision"], 8);
    assert_eq!(summary["proof"]["unknown_outcome_reconciled"], true);
    assert_eq!(summary["measurements"]["engine_opens"], 1);
    assert_eq!(summary["measurements"]["authoring_rpc_calls"], 2);
    assert_eq!(summary["measurements"]["provider_tokens"], Value::Null);
    assert!(summary["proof"]["slot_bytes"].as_u64().expect("slot bytes") > 0);
}
