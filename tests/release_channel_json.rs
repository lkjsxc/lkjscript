#![allow(clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn driver_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/release-channel/driver.py")
}

fn run_driver() -> Output {
    Command::new("python3")
        .arg(driver_path())
        .arg(env!("CARGO_BIN_EXE_lkjscript"))
        .arg(env!("CARGO_BIN_EXE_lkjscriptd"))
        .output()
        .expect("run release-channel replay driver")
}

fn successful_summary(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "release-channel driver failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "driver stderr must stay empty");
    serde_json::from_slice(&output.stdout).expect("release-channel summary JSON")
}

#[test]
fn real_cli_release_channel_replay_repairs_runs_renames_and_restarts() {
    let summary = successful_summary(&run_driver());
    assert_eq!(
        summary["revisions"],
        json!({"incomplete": 1, "repaired": 2, "renamed": 3})
    );
    assert_eq!(summary["repair"]["rejected_code"], "type_mismatch");
    assert_eq!(summary["repair"]["allocator_rollback"], true);
    assert_eq!(summary["repair"]["identity_preserved"], true);
    assert_eq!(summary["repair"]["owner_preserved"], true);
    assert_eq!(summary["repair"]["body_position_preserved"], true);
    assert_eq!(summary["repair"]["output_zero_preserved"], true);
    assert_eq!(summary["repair"]["use_sites_preserved"], true);
    assert_eq!(summary["repair"]["change"], "operation_refined");
    assert_eq!(
        summary["history"]["names"],
        json!(["rollout_steps", "rollout_steps", "steps"])
    );
    assert_eq!(summary["history"]["rename_identity_preserved"], true);
    assert_eq!(
        summary["laziness"]["selected_expensive_branch_error"],
        "execution_fuel_exhausted"
    );
    assert_eq!(summary["restart"]["revision_one_incomplete"], true);
    assert_eq!(summary["restart"]["revisions_two_three_equal"], true);
    assert_eq!(summary["restart"]["identities_persisted"], true);
    assert_eq!(summary["counts"]["explicit_draft_symbols"], 111);
    assert_eq!(summary["counts"]["selected_bindings"], 38);
    assert_eq!(summary["counts"]["rejected_proposals"], 1);
    assert_eq!(summary["contracts"]["task_roots"], 12);
    assert_eq!(summary["provider_telemetry"]["available"], false);
    assert_eq!(summary["interaction"]["daemon_processes"], 2);
    assert_eq!(summary["interaction"]["lifecycle_cli_launches"], 2);
    assert_eq!(summary["shutdown"], "acknowledged");
}
