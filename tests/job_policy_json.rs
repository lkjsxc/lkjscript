#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use lkjscript::machine::{RequestEnvelope, ResponseEnvelope};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn driver_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/job-policy/driver.py")
}

fn run_driver(metrics: Option<&Path>) -> Output {
    let mut command = Command::new("python3");
    command
        .arg(driver_path())
        .arg(env!("CARGO_BIN_EXE_lkjscript"));
    if let Some(path) = metrics {
        command.arg(path);
    }
    command.output().expect("run job-policy driver")
}

fn successful_summary(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "job-policy driver failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "driver stderr must stay empty");
    serde_json::from_slice(&output.stdout).expect("job-policy summary JSON")
}

#[test]
fn real_cli_job_policy_repairs_renames_runs_and_restarts() {
    let output = run_driver(None);
    let summary = successful_summary(&output);
    assert_eq!(
        summary["revisions"],
        json!({"incomplete": 1, "repaired": 2, "renamed": 3})
    );
    assert_eq!(summary["repair"]["rejected_code"], "type_mismatch");
    assert_eq!(summary["repair"]["allocator_rollback"], true);
    assert_eq!(summary["repair"]["change"], "operation_refined");
    assert_eq!(summary["rename"]["before"], "memory");
    assert_eq!(summary["rename"]["after"], "memory_units");
    assert_eq!(summary["oracles"]["cases_a_through_h"], true);
    assert_eq!(summary["oracles"]["lazy_unselected_work"], true);
    assert_eq!(summary["oracles"]["restart"], true);
    assert_eq!(summary["oracles"]["exact_named_ids"], true);
    assert_eq!(summary["counts"]["selected_bindings"], 32);
    assert_eq!(summary["counts"]["expected_rejected_proposals"], 1);
    assert_eq!(summary["reopen"], "passed on every direct command");
}

#[test]
#[ignore = "manual release job-policy interaction-cost measurement"]
fn job_policy_agent_interaction_cost_measurement() {
    let temporary = tempfile::tempdir().expect("measurement directory");
    let metrics_path = temporary.path().join("job-policy-metrics.json");
    let output = run_driver(Some(&metrics_path));
    let summary = successful_summary(&output);
    let metrics: Value =
        serde_json::from_slice(&std::fs::read(&metrics_path).expect("job-policy metrics file"))
            .expect("job-policy metrics JSON");
    assert_eq!(metrics["summary"], summary);

    let records = metrics["measurements"]
        .as_array()
        .expect("measurement records");
    let mut rows = Vec::new();
    let mut json_request_total = 0_u64;
    let mut json_response_total = 0_u64;
    let mut ipc_request_frame_total = 0_usize;
    let mut ipc_response_frame_total = 0_usize;
    let mut wall_total = 0_u64;
    let mut counted_rows = 0_usize;
    let mut lifecycle_rows = 0_usize;

    for record in records {
        let request: RequestEnvelope =
            serde_json::from_value(record["request"].clone()).expect("typed measured request");
        let response: ResponseEnvelope =
            serde_json::from_value(record["response"].clone()).expect("typed measured response");
        assert_eq!(request.request_id, response.request_id);
        let ipc_request_frame = serde_json::to_vec(&request)
            .expect("measured request JSON")
            .len()
            + u32::BITS as usize / 8;
        let ipc_response_frame = serde_json::to_vec(&response)
            .expect("measured response JSON")
            .len()
            + u32::BITS as usize / 8;
        let counted = record["counted"].as_bool().expect("counted flag");
        if counted {
            counted_rows += 1;
            json_request_total += record["json_request_bytes"]
                .as_u64()
                .expect("JSON request bytes");
            json_response_total += record["json_response_bytes"]
                .as_u64()
                .expect("JSON response bytes");
            ipc_request_frame_total += ipc_request_frame;
            ipc_response_frame_total += ipc_response_frame;
            wall_total += record["elapsed_nanoseconds"]
                .as_u64()
                .expect("CLI wall nanoseconds");
        } else {
            lifecycle_rows += 1;
        }

        let response_json = &record["response"]["response"];
        let kind = response_json["kind"].as_str().expect("response kind");
        let semantic_outcome = if kind == "error" {
            response_json["data"]["code"]
                .as_str()
                .expect("typed error code")
                .to_owned()
        } else {
            "success".to_owned()
        };
        let returned_items = match kind {
            "query_batch_result" => response_json["data"]["results"]
                .as_array()
                .map_or(0, Vec::len),
            "describe_schema" => response_json["data"]["data"]["definitions"]
                .as_array()
                .map_or(1, Vec::len),
            _ => 1,
        };
        let selected_bindings = if kind == "transaction_receipt" {
            response_json["data"]["returned_bindings"]
                .as_array()
                .map_or(0, Vec::len)
        } else {
            0
        };
        rows.push(json!({
            "purpose": record["purpose"],
            "counted": counted,
            "json_request_bytes": record["json_request_bytes"],
            "json_response_bytes": record["json_response_bytes"],
            "ipc_request_frame_bytes": ipc_request_frame,
            "ipc_response_frame_bytes": ipc_response_frame,
            "cli_engine_wall_nanoseconds": record["elapsed_nanoseconds"],
            "semantic_outcome": semantic_outcome,
            "returned_items": returned_items,
            "selected_bindings": selected_bindings,
        }));
    }

    assert_eq!(
        summary["interaction"]["engine_opens"]
            .as_u64()
            .expect("engine opens") as usize,
        counted_rows
    );
    assert_eq!(
        summary["interaction"]["cli_launches"]
            .as_u64()
            .expect("CLI launches") as usize,
        counted_rows
    );
    assert_eq!(lifecycle_rows, 0);
    assert_eq!(records.len(), counted_rows + lifecycle_rows);
    assert_eq!(
        summary["interaction"]["json_request_bytes"],
        json!(json_request_total)
    );
    assert_eq!(
        summary["interaction"]["json_response_bytes"],
        json!(json_response_total)
    );
    assert_eq!(
        summary["interaction"]["cli_engine_wall_nanoseconds"],
        json!(wall_total)
    );
    let report = json!({
        "rows": rows,
        "totals": {
            "json_request_bytes": summary["interaction"]["json_request_bytes"],
            "json_response_bytes": summary["interaction"]["json_response_bytes"],
            "ipc_request_frame_bytes": ipc_request_frame_total,
            "ipc_response_frame_bytes": ipc_response_frame_total,
            "cli_launches": summary["interaction"]["cli_launches"],
            "engine_opens": summary["interaction"]["engine_opens"],
            "connections": 0,
            "cli_engine_wall_nanoseconds": summary["interaction"]["cli_engine_wall_nanoseconds"],
        },
        "summary": summary,
        "provider_telemetry": {
            "available": false,
            "tokens": null,
            "api_cost": null,
        },
    });
    println!("JOB_POLICY_AGENT_COST {}", report);
}
