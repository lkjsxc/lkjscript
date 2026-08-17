#![allow(clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

fn driver_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/agent-maintenance/driver.py")
}

#[test]
fn semantic_workbench_completes_the_eight_revision_maintenance_corpus() {
    let output = Command::new("python3")
        .arg(driver_path())
        .arg(env!("CARGO_BIN_EXE_lkjscript"))
        .output()
        .expect("run agent-maintenance driver");
    assert!(
        output.status.success(),
        "agent-maintenance driver failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "driver stderr must stay empty");
    let summary: Value =
        serde_json::from_slice(&output.stdout).expect("agent-maintenance summary JSON");
    assert_eq!(
        summary["revisions"],
        json!({
            "incomplete": 1,
            "repaired": 2,
            "extended": 3,
            "refactored": 4,
            "renamed": 5,
            "debug_trap": 6,
            "debug_fixed": 7,
            "migrated": 8
        })
    );
    assert_eq!(summary["oracles"]["invalid_repair_atomic"], true);
    assert_eq!(summary["oracles"]["identity_preserving_repair"], true);
    assert_eq!(summary["oracles"]["extended_and_refactored_score"], 27);
    assert_eq!(summary["oracles"]["debug_trap"]["code"], "runtime_trap");
    assert_eq!(summary["oracles"]["blocked_delete"], "delete_blocked");
    assert_eq!(summary["oracles"]["restart"], true);
    assert_eq!(summary["interface_comparison"]["receipts_identical"], true);
    assert_eq!(
        summary["interface_comparison"]["baseline_json_accepted"],
        true
    );
    assert_eq!(
        summary["interface_comparison"]["candidate_document_accepted"],
        true
    );
    assert!(
        summary["interface_comparison"]["candidate_document_bytes"]
            .as_u64()
            .expect("candidate document bytes")
            < summary["interface_comparison"]["baseline_request_bytes"]
                .as_u64()
                .expect("baseline request bytes")
    );
    assert_eq!(summary["provider_telemetry"], "unavailable");
    assert_eq!(summary["reopen"], "passed on every direct command");
}
