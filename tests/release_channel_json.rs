#![allow(clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn driver_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/release-channel/driver.py")
}

fn run_driver(authoring_mode: &str) -> Output {
    let mut command = Command::new("python3");
    command
        .arg(driver_path())
        .arg(env!("CARGO_BIN_EXE_lkjscript"))
        .env("LKJSCRIPT_AUTHORING_MODE", authoring_mode);
    command.output().expect("run release-channel replay driver")
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

fn without_workspace(value: &Value, workspace: &str) -> Value {
    match value {
        Value::String(text) => Value::String(text.replace(workspace, "<workspace>")),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| without_workspace(item, workspace))
                .collect(),
        ),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), without_workspace(value, workspace)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

#[test]
fn real_cli_release_channel_replay_repairs_runs_renames_and_restarts() {
    let summary = successful_summary(&run_driver("inline"));
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
    assert_eq!(summary["counts"]["explicit_draft_symbols"], 67);
    assert_eq!(summary["counts"]["selected_bindings"], 38);
    assert_eq!(summary["counts"]["rejected_proposals"], 1);
    assert_eq!(summary["contracts"]["task_roots"], 11);
    assert_eq!(summary["provider_telemetry"]["available"], false);
    assert_eq!(summary["interaction"]["connections"], 0);
    assert_eq!(summary["reopen"], "passed on every direct command");
}

#[test]
fn explicit_and_inline_release_channel_modes_keep_equal_work_and_reduce_scaffolding() {
    let explicit = successful_summary(&run_driver("explicit"));
    let inline = successful_summary(&run_driver("inline"));

    assert_eq!(explicit["counts"]["explicit_draft_symbols"], 111);
    assert_eq!(inline["counts"]["explicit_draft_symbols"], 67);
    assert_eq!(inline["proposals"]["inline_removed_symbols"], 44);
    assert_eq!(
        explicit["proposals"]["initial_compact_payload_bytes"],
        22_062
    );
    assert_eq!(inline["proposals"]["initial_compact_payload_bytes"], 17_974);
    assert_eq!(
        explicit["proposals"]["rows"][0]["json_request_bytes"],
        22_248
    );
    assert_eq!(inline["proposals"]["rows"][0]["json_request_bytes"], 18_160);

    for field in [
        "initial_operations",
        "selected_bindings",
        "created_durable_entities",
        "canonical_nodes",
        "rejected_proposals",
    ] {
        assert_eq!(
            explicit["counts"][field], inline["counts"][field],
            "{field}"
        );
    }
    assert_eq!(explicit["artifacts"], inline["artifacts"]);
    assert_eq!(explicit["repair"], inline["repair"]);
    assert_eq!(explicit["history"], inline["history"]);
    let explicit_workspace = explicit["workspace"].as_str().expect("explicit workspace");
    let inline_workspace = inline["workspace"].as_str().expect("inline workspace");
    assert_eq!(
        without_workspace(&explicit["laziness"], explicit_workspace),
        without_workspace(&inline["laziness"], inline_workspace)
    );
    assert_eq!(
        without_workspace(&explicit["run_results"], explicit_workspace),
        without_workspace(&inline["run_results"], inline_workspace)
    );
    assert_eq!(explicit["restart"], inline["restart"]);
    assert_eq!(explicit["contracts"], inline["contracts"]);
    assert!(
        explicit["interaction"]["json_response_bytes"]
            .as_u64()
            .expect("explicit response bytes")
            > 0
    );
    assert!(
        inline["interaction"]["json_response_bytes"]
            .as_u64()
            .expect("inline response bytes")
            > 0
    );
    assert_eq!(
        explicit["interaction"]["json_request_bytes"]
            .as_u64()
            .expect("explicit bytes")
            - inline["interaction"]["json_request_bytes"]
                .as_u64()
                .expect("inline bytes"),
        4_088
    );
}
