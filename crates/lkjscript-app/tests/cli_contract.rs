#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::process::Command;

#[test]
fn help_and_optimizing_metrics_expose_the_current_contract() {
    let binary = env!("CARGO_BIN_EXE_lkjscript");
    let help = Command::new(binary)
        .arg("--help")
        .output()
        .expect("run CLI help");
    assert!(help.status.success());
    assert!(help.stderr.is_empty());
    let help = String::from_utf8(help.stdout).expect("help is UTF-8");
    assert!(help.contains("--engine vm|auto|baseline-jit|optimizing-jit"));
    assert!(help.contains("default: auto at 64 function entries"));

    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/optimizing-loop.lkjscript");
    let metrics = std::env::temp_dir().join(format!(
        "lkjscript-optimizing-metrics-{}-{}.json",
        std::process::id(),
        std::thread::current().name().unwrap_or("cli")
    ));
    let output = Command::new(binary)
        .args(["run", "--engine", "optimizing-jit"])
        .arg(&fixture)
        .env("LKJSCRIPT_METRICS", "1")
        .env("LKJSCRIPT_METRICS_FILE", &metrics)
        .output()
        .expect("run optimizing CLI metrics fixture");
    assert!(output.status.success(), "stderr={:?}", output.stderr);
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    let json = std::fs::read_to_string(&metrics).expect("read metrics output");
    std::fs::remove_file(&metrics).expect("remove metrics output");
    for field in [
        "\"optimization_work_units\":",
        "\"instruction_growth\":",
        "\"cleanup_removed_instructions\":",
        "\"optimizing_passes\":",
        "\"optimization_discovery_passes\":",
        "\"optimization_checker_passes\":",
        "\"optimization_reconstruction_passes\":",
        "\"optimization_cleanup_passes\":",
        "\"optimization_validation_passes\":",
        "\"optimization_certificate_bytes_estimate\":",
        "\"optimization_metadata_bytes_estimate\":",
        "\"certificate_bytes_estimate\":",
    ] {
        assert!(json.contains(field), "missing metrics field {field}");
    }
    assert!(!json.contains("\"optimization_certificate_bytes\":"));
    assert!(!json.contains("\"optimization_metadata_bytes\":"));
}
