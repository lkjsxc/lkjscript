#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "the black-box test harness uses panic-on-failure assertions"
)]

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
        output.stdout.len() < 16 * 1024,
        "success output is excessive"
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("machine JSON");
    assert_eq!(value["ok"], true);
    value
}

#[test]
fn orientation_tests_build_and_artifact_inspection_are_public_and_compact() {
    let help = success(&["help"]);
    assert!(
        help["result"]["usage"]
            .as_str()
            .expect("usage")
            .contains("component inspect")
    );

    let orientation = success(&["--project", "applications/lkjournal", "project", "orient"]);
    assert_eq!(
        orientation["result"]["modules"].as_array().unwrap().len(),
        3
    );
    assert_eq!(
        orientation["result"]["targets"].as_array().unwrap().len(),
        2
    );

    let targets = success(&["--project", "applications/lkjournal", "target", "list"]);
    assert_eq!(targets["result"]["targets"].as_array().unwrap().len(), 2);
    let component = success(&[
        "--project",
        "applications/lkjournal",
        "component",
        "inspect",
        "service.Web",
    ]);
    assert_eq!(component["result"]["owner"]["declaration"], "Web");
    assert_eq!(
        component["result"]["requirements"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
    assert_eq!(component["result"]["ports"].as_array().unwrap().len(), 1);

    let tests = success(&["--project", "applications/lkjournal", "package", "test"]);
    assert_eq!(tests["result"]["passed"], 11);
    assert_eq!(tests["result"]["differential"], "equal");

    let temporary = tempfile::TempDir::new().expect("temporary output");
    let artifact = temporary.path().join("application.lkja");
    let artifact_text = artifact.to_str().expect("UTF-8 path");
    let first = success(&[
        "--project",
        "applications/lkjournal",
        "package",
        "build",
        "--output",
        artifact_text,
    ]);
    assert_eq!(first["result"]["publication"], "published");
    let repeated = success(&[
        "--project",
        "applications/lkjournal",
        "package",
        "build",
        "--output",
        artifact_text,
    ]);
    assert_eq!(repeated["result"]["publication"], "existing_equal");
    let inspection = success(&["artifact", "inspect", artifact_text]);
    assert_eq!(inspection["result"]["targets"].as_array().unwrap().len(), 2);
}

#[test]
fn backup_restore_and_deep_reconstruction_use_the_current_authority() {
    let temporary = tempfile::TempDir::new().expect("temporary backup");
    let backup = temporary.path().join("backup");
    let restored = temporary.path().join("restored");
    success(&[
        "--project",
        "applications/lkjournal",
        "project",
        "backup",
        "--output",
        backup.to_str().expect("UTF-8 backup path"),
    ]);
    success(&[
        "project",
        "restore-backup",
        "--backup",
        backup.to_str().expect("UTF-8 backup path"),
        "--output",
        restored.to_str().expect("UTF-8 restore path"),
    ]);
    let doctor = success(&[
        "--project",
        restored.to_str().expect("UTF-8 restore path"),
        "project",
        "doctor",
        "--deep",
    ]);
    assert_eq!(doctor["result"]["valid"], true);
    assert_eq!(doctor["result"]["records_checked"], 8);
}

#[test]
fn predecessor_project_rejects_at_the_public_boundary() {
    let temporary = tempfile::TempDir::new().expect("temporary predecessor");
    std::fs::write(temporary.path().join("lkjscript.package.json"), b"{}\n")
        .expect("package marker");
    std::fs::create_dir(temporary.path().join(".lkjscript")).expect("predecessor marker parent");
    std::fs::write(temporary.path().join(".lkjscript/project"), b"old\n")
        .expect("predecessor marker");
    let output = command(&["--project", path(temporary.path()), "project", "orient"]);
    assert_eq!(output.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&output.stdout).expect("failure JSON");
    assert_eq!(value["error"]["code"], "source_predecessor_rejected");
}

fn path(value: &Path) -> &str {
    value.to_str().expect("UTF-8 temporary path")
}
