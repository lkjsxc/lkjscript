#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::process::Command;

#[path = "canonical/application_control.rs"]
mod application_control;
#[path = "canonical/application_control/support.rs"]
mod application_control_support;
#[path = "canonical/process_cells.rs"]
mod process_cells;
#[path = "canonical/session_broker.rs"]
mod session_broker;

#[test]
fn help_cli_and_metrics_expose_one_product_execution_path() {
    let binary = env!("CARGO_BIN_EXE_lkjscript");
    let help = Command::new(binary)
        .arg("--help")
        .output()
        .expect("run CLI help");
    assert!(help.status.success());
    assert!(help.stderr.is_empty());
    let help = String::from_utf8(help.stdout).expect("help is UTF-8");
    assert!(help.contains("run <file.lkjscript> [--] [script-args...]"));
    assert!(help.contains("one baseline-native attempt, then VM fallback before entry"));
    assert!(help.contains("memory inventory [--json]"));
    for removed in [
        "--engine",
        "--auto-jit-threshold",
        "--disable-auto-jit",
        "--resource-profile",
    ] {
        assert!(
            !help.contains(removed),
            "removed flag remains in help: {removed}"
        );
    }

    let description = Command::new(binary)
        .args(["describe", "--json"])
        .output()
        .expect("run JSON describe");
    assert!(description.status.success());
    let description = String::from_utf8(description.stdout).expect("describe is UTF-8");
    assert!(description.contains("\"execution_path\":\"baseline-native-with-vm-fallback\""));
    assert!(!description.contains("\"engines\":"));

    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/optimizing-loop.lkjscript");
    let metrics = std::env::temp_dir().join(format!(
        "lkjscript-product-metrics-{}-{}.json",
        std::process::id(),
        std::thread::current().name().unwrap_or("cli")
    ));
    let output = Command::new(binary)
        .arg("run")
        .arg(&fixture)
        .env("LKJSCRIPT_METRICS", "1")
        .env("LKJSCRIPT_METRICS_FILE", &metrics)
        .output()
        .expect("run scalar CLI metrics fixture");
    assert!(output.status.success(), "stderr={:?}", output.stderr);
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    let json = std::fs::read_to_string(&metrics).expect("read metrics output");
    std::fs::remove_file(&metrics).expect("remove metrics output");
    for field in [
        "\"execution_path\":\"baseline-native\"",
        "\"fallback_reason\":null",
        "\"native_entered\":true",
        "\"preflight\":",
        "\"lower\":",
        "\"install\":",
        "\"prepare\":",
        "\"native\":",
        "\"vm\":",
        "\"total\":",
    ] {
        assert!(json.contains(field), "missing metrics field {field}");
    }
    for removed in [
        "\"engine\":",
        "configured_auto_threshold",
        "auto_enabled",
        "auto_threshold",
        "\"tier\":",
        "\"jit\":",
    ] {
        assert!(
            !json.contains(removed),
            "removed metrics field remains: {removed}"
        );
    }

    let fallback =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/examples/hello/main.lkjscript");
    let output = Command::new(binary)
        .arg("run")
        .arg(&fallback)
        .env("LKJSCRIPT_METRICS", "1")
        .env("LKJSCRIPT_METRICS_FILE", &metrics)
        .output()
        .expect("run fallback CLI metrics fixture");
    assert!(output.status.success(), "stderr={:?}", output.stderr);
    assert_eq!(output.stdout, b"3628800");
    let fallback_json = std::fs::read_to_string(&metrics).expect("read fallback metrics output");
    std::fs::remove_file(&metrics).expect("remove fallback metrics output");
    assert!(fallback_json.contains("\"execution_path\":\"vm-fallback\""));
    assert!(fallback_json.contains("\"fallback_reason\":\"unsupported-shape\""));
    assert!(fallback_json.contains("\"native_entered\":false"));

    for arguments in [
        vec!["run", "--engine", "vm"],
        vec!["run", "--auto-jit-threshold", "1"],
        vec!["run", "--disable-auto-jit"],
        vec!["run", "--resource-profile", "default"],
    ] {
        let removed = Command::new(binary)
            .args(&arguments)
            .arg(&fixture)
            .output()
            .expect("run removed option");
        assert!(
            !removed.status.success(),
            "removed option accepted: {arguments:?}"
        );
        assert!(String::from_utf8(removed.stderr)
            .expect("removed option diagnostic is UTF-8")
            .contains(&format!("unknown run option: {}", arguments[1])));
    }
}

#[test]
fn runtime_topology_scheduler_and_plan_are_exact_public_evidence() {
    let binary = env!("CARGO_BIN_EXE_lkjscript");
    for (operation, schema) in [
        (
            &["runtime", "topology", "--json"][..],
            "lkjscript.runtime-topology",
        ),
        (
            &["runtime", "host-scheduler", "--json"][..],
            "lkjscript.host-scheduler",
        ),
        (
            &[
                "runtime",
                "plan",
                "--json",
                "--parallelism",
                "2",
                "--tasks",
                "4",
            ][..],
            "lkjscript.execution-resource-plan",
        ),
    ] {
        let output = Command::new(binary)
            .args(operation)
            .output()
            .expect("run runtime evidence command");
        assert!(output.status.success(), "stderr={:?}", output.stderr);
        assert!(output.stderr.is_empty());
        let json = String::from_utf8(output.stdout).expect("runtime JSON is UTF-8");
        assert!(json.contains(&format!("\"schema\":\"{schema}\"")));
        assert!(json.contains(&format!(
            "\"contract\":\"{}\"",
            lkjscript_contracts::SEMANTIC_RESOURCE_PLANE_DIGEST
        )));
        assert!(json.contains("\"snapshot\":"));
    }
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
    assert!(json.contains("\"identity\":\"enum\""));
    assert!(json.contains("\"current_trace_fields\":\"none\""));
    assert!(json.contains("current deterministic storage; unsupported shapes reject"));
    assert!(json.contains("verified static image data or execution-owned unique store"));

    let explain = Command::new(binary)
        .args(["memory", "explain", "byte-vector"])
        .output()
        .expect("run memory explanation");
    assert!(explain.status.success());
    assert!(explain.stderr.is_empty());
    let text = String::from_utf8(explain.stdout).expect("explanation is UTF-8");
    assert!(text.contains("memory-identity=byte-vector"));
    assert!(text.contains("current exact evaluator/VM/forced-native byte-vector subset"));
}
