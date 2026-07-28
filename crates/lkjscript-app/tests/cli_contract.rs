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
    assert!(help.contains("memory inventory [--json]"));

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

#[test]
fn memory_inventory_and_explain_are_deterministic_public_evidence() {
    let binary = env!("CARGO_BIN_EXE_lkjscript");
    let inventory = Command::new(binary)
        .args(["memory", "inventory", "--json"])
        .output()
        .expect("run memory inventory");
    assert!(inventory.status.success());
    assert!(inventory.stderr.is_empty());
    let json = String::from_utf8(inventory.stdout).expect("inventory is UTF-8");
    assert!(json.contains("\"schema\":\"lkjscript.memory-obligations\""));
    assert!(json.contains("\"identity\":\"gc-heap\""));
    assert!(json.contains("\"current_trace_fields\":\"HeapObj::trace from exact roots\""));
    assert!(json.contains("static bytecode data or evaluator/VM unique store"));

    let explain = Command::new(binary)
        .args(["memory", "explain", "byte-vector"])
        .output()
        .expect("run memory explanation");
    assert!(explain.status.success());
    assert!(explain.stderr.is_empty());
    let text = String::from_utf8(explain.stdout).expect("explanation is UTF-8");
    assert!(text.contains("memory-identity=byte-vector"));
    assert!(text.contains("current exact evaluator/VM/forced-native byte-vector subset"));

    let traced = Command::new(binary)
        .args(["memory", "traced", "--json"])
        .output()
        .expect("run memory tracing ratchet");
    assert!(traced.status.success());
    assert!(traced.stderr.is_empty());
    let json = String::from_utf8(traced.stdout).expect("tracing ratchet is UTF-8");
    assert!(json.contains("\"schema\":\"lkjscript.memory-tracing-ratchet\""));
    assert!(json.contains("\"identity\":\"buf\",\"heap_variant\":\"Buf\""));
    assert!(json.contains("\"identity\":\"symbol\",\"heap_variant\":\"Symbol\""));
    assert_eq!(json.matches("\"heap_variant\":").count(), 9);
}
