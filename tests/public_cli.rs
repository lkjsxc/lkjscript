#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "the black-box test harness uses panic-on-failure assertions"
)]

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const APPLICATION: &str = "applications/lkjournal";

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lkjscript"))
}

fn command(arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run public CLI")
}

fn success(arguments: &[&str]) -> Value {
    let output = command(arguments);
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stderr.is_empty());
    assert!(
        output.stdout.len() < 64 * 1024,
        "success output is excessive"
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("machine JSON");
    assert_eq!(value["ok"], true);
    assert_eq!(value["contract_version"], 2);
    value
}

#[test]
fn semantic_orientation_query_test_build_and_artifact_paths_are_public_and_compact() {
    let help = success(&["help"]);
    assert!(
        help["result"]["usage"]
            .as_str()
            .expect("usage")
            .contains("semantic <command>")
    );
    assert!(
        help["result"]["commands"]
            .as_array()
            .expect("commands")
            .iter()
            .any(|command| command == "apply")
    );

    let orientation = success(&[
        "--project",
        APPLICATION,
        "semantic",
        "orient",
        "--limit",
        "10",
    ]);
    assert_eq!(orientation["result"]["authority"], "typed_semantic_graph");
    assert_eq!(orientation["result"]["module_count"], 3);
    assert_eq!(orientation["result"]["target_count"], 2);
    assert!(
        orientation["result"]["revision"]
            .as_str()
            .expect("revision")
            .starts_with("rev_")
    );

    let targets = success(&[
        "--project",
        APPLICATION,
        "semantic",
        "targets",
        "--limit",
        "5",
    ]);
    assert_eq!(targets["result"]["items"].as_array().unwrap().len(), 2);

    let found = success(&[
        "--project",
        APPLICATION,
        "semantic",
        "find",
        "Web",
        "--exact",
        "--limit",
        "5",
    ]);
    let component_id = found["result"]["items"][0]["id"]
        .as_str()
        .expect("component id");
    let component = success(&[
        "--project",
        APPLICATION,
        "semantic",
        "show",
        component_id,
        "--body",
    ]);
    assert_eq!(component["result"]["kind"], "component");
    assert_eq!(
        component["result"]["semantic"]["data"]["requirements"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
    assert_eq!(
        component["result"]["semantic"]["data"]["ports"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let tests = success(&["--project", APPLICATION, "semantic", "test"]);
    assert_eq!(tests["result"]["passed"], 11);
    assert_eq!(tests["result"]["differential"], "equal");

    let temporary = tempfile::TempDir::new().expect("temporary output");
    let artifact = temporary.path().join("application.lkja");
    let artifact_text = artifact.to_str().expect("UTF-8 path");
    let first = success(&[
        "--project",
        APPLICATION,
        "semantic",
        "build",
        "--output",
        artifact_text,
    ]);
    assert_eq!(first["result"]["publication"], "published");
    let repeated = success(&[
        "--project",
        APPLICATION,
        "semantic",
        "build",
        "--output",
        artifact_text,
    ]);
    assert_eq!(repeated["result"]["publication"], "unchanged");
    let inspection = success(&["semantic", "artifact-inspect", artifact_text]);
    assert_eq!(inspection["result"]["targets"].as_array().unwrap().len(), 2);
}

#[test]
fn graph_backup_restore_and_deep_reconstruction_are_exact() {
    let temporary = tempfile::TempDir::new().expect("temporary backup");
    let backup = temporary.path().join("backup.lkjb");
    let restored = temporary.path().join("restored");
    std::fs::create_dir(&restored).expect("restore project directory");
    let backup_result = success(&[
        "--project",
        APPLICATION,
        "semantic",
        "backup",
        "--output",
        backup.to_str().expect("UTF-8 backup path"),
    ]);
    assert_eq!(backup_result["result"]["receipt"]["drafts"], 0);
    let restore = success(&[
        "semantic",
        "restore",
        "--backup",
        backup.to_str().expect("UTF-8 backup path"),
        "--output",
        restored.to_str().expect("UTF-8 restore path"),
    ]);
    assert_eq!(
        restore["result"]["receipt"]["revision"],
        backup_result["result"]["receipt"]["revision"]
    );
    let doctor = success(&[
        "--project",
        restored.to_str().expect("UTF-8 restore path"),
        "semantic",
        "doctor",
        "--deep",
    ]);
    assert_eq!(doctor["result"]["valid"], true);
    assert_eq!(doctor["result"]["revisions_checked"], 1);
    let tests = success(&[
        "--project",
        restored.to_str().expect("UTF-8 restore path"),
        "semantic",
        "test",
    ]);
    assert_eq!(tests["result"]["passed"], 11);
    assert_eq!(tests["result"]["differential"], "equal");
}

#[test]
fn broad_transaction_results_are_compact_and_expandable() {
    let temporary = tempfile::TempDir::new().expect("temporary graph authority");
    let standard = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("applications/lkjournal/dependencies/standard.lkja");
    success(&[
        "--project",
        path(temporary.path()),
        "semantic",
        "import",
        "--artifact",
        path(&standard),
    ]);
    let status = success(&["--project", path(temporary.path()), "semantic", "status"]);
    let operations = (1u128..=100)
        .map(|ordinal| {
            serde_json::json!({
                "operation_kind": "create_module",
                "id": format!("mod_{ordinal:032x}"),
                "name": format!("bounded.module{ordinal:03}"),
            })
        })
        .collect::<Vec<_>>();
    let request = serde_json::json!({
        "contract_version": 1,
        "graph_contract": "lkjscript-meaning-graph-1",
        "repository_id": status["result"]["repository_id"],
        "base_revision": status["result"]["revision"],
        "operations": operations,
        "budget": {
            "maximum_operations": 100,
            "maximum_work": 10000,
            "maximum_affected_owners": 100,
        },
    });
    let request_path = temporary.path().join("transaction.json");
    std::fs::write(
        &request_path,
        serde_json::to_vec(&request).expect("encode transaction"),
    )
    .expect("write transaction");
    let applied = success(&[
        "--project",
        path(temporary.path()),
        "semantic",
        "apply",
        "--request-file",
        path(&request_path),
    ]);
    assert_eq!(applied["status"], "accepted_change");
    assert_eq!(applied["result"]["affected_owner_count"], 100);
    assert_eq!(
        applied["result"]["affected_owners"]
            .as_array()
            .expect("affected owners")
            .len(),
        64
    );
    assert_eq!(applied["result"]["affected_owners_truncated"], true);
    assert!(
        applied["result"]["receipt"]["expansion"]
            .as_str()
            .expect("receipt expansion")
            .starts_with("semantic revision-show rev_")
    );
}

#[test]
fn predecessor_source_authority_rejects_at_the_public_boundary() {
    let temporary = tempfile::TempDir::new().expect("temporary predecessor");
    std::fs::write(temporary.path().join("lkjscript.package.json"), b"{}\n")
        .expect("package marker");
    std::fs::create_dir_all(temporary.path().join(".lkjscript/source-v1"))
        .expect("predecessor marker parent");
    std::fs::write(
        temporary.path().join(".lkjscript/source-v1/HEAD.json"),
        b"{}\n",
    )
    .expect("predecessor marker");
    let output = command(&["--project", path(temporary.path()), "semantic", "orient"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).expect("failure JSON");
    assert_eq!(
        value["error"]["code"],
        "semantic_predecessor_source_rejected"
    );
}

fn path(value: &Path) -> &str {
    value.to_str().expect("UTF-8 temporary path")
}
