#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::process::Command;

fn read_metrics(path: &std::path::Path) -> serde_json::Value {
    let line = std::fs::read_to_string(path).expect("read metrics output");
    let json = line
        .strip_prefix("LKJSCRIPT_METRICS ")
        .expect("metrics marker");
    serde_json::from_str(json).expect("metrics are valid JSON")
}

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
    assert!(!help.contains("semantic"));
    assert!(!help.contains("runtime topology"));
    assert!(!help.contains("host-scheduler"));
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
    assert!(!description.contains("semantic_operations"));
    assert!(!description.contains("platform_revision"));

    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scalar-loop.lkjscript");
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
    let json = read_metrics(&metrics);
    std::fs::remove_file(&metrics).expect("remove metrics output");
    assert_eq!(json["execution_path"].as_str(), Some("baseline-native"));
    assert!(json.get("fallback_reason").is_none());
    assert!(json["native_decline"].is_null());
    assert_eq!(json["native_entered"].as_bool(), Some(true));
    for field in [
        "preflight",
        "lower",
        "install",
        "prepare",
        "native",
        "vm",
        "total",
    ] {
        assert!(
            json["timings_ns"][field].is_number(),
            "missing timing {field}"
        );
    }
    let artifact = &json["native_artifact"];
    assert_eq!(
        artifact["availability"].as_str(),
        Some("published-installed-object")
    );
    assert_eq!(artifact["objects"].as_u64(), Some(1));
    for field in ["code_bytes", "metadata_bytes", "mapped_bytes"] {
        assert!(artifact[field].as_u64().is_some_and(|value| value > 0));
    }
    let runtime = &json["native_runtime"];
    assert_eq!(runtime["counter_semantics"].as_str(), Some("saturating"));
    assert!(runtime["entries"].as_u64().is_some_and(|value| value > 0));
    assert_eq!(runtime["invocations"].as_u64(), Some(1));
    for removed in ["engine", "tier", "jit", "fallback_reason"] {
        assert!(
            json.get(removed).is_none(),
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
    let fallback_json = read_metrics(&metrics);
    std::fs::remove_file(&metrics).expect("remove fallback metrics output");
    assert_eq!(
        fallback_json["execution_path"].as_str(),
        Some("vm-fallback")
    );
    assert!(fallback_json.get("fallback_reason").is_none());
    assert_eq!(
        fallback_json["native_decline"]["stage"].as_str(),
        Some("lowering")
    );
    assert_eq!(
        fallback_json["native_decline"]["code"].as_str(),
        Some("unsupported-type")
    );
    assert!(fallback_json["native_decline"]["detail"].is_string());
    assert_eq!(fallback_json["native_entered"].as_bool(), Some(false));
    assert!(fallback_json["native_artifact"].is_null());

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
fn package_capability_denial_prevents_host_effects() {
    let binary = env!("CARGO_BIN_EXE_lkjscript");
    let root = std::env::temp_dir().join(format!(
        "lkjscript-cli-capability-denial-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("cli")
    ));
    std::fs::create_dir_all(&root).expect("create capability fixture");
    let entry = root.join("main.lkjscript");
    std::fs::write(
        &entry,
        concat!(
            "main/\nsig/\ninputs/\ncapability/\nstdio\n/capability\n/inputs\n",
            "output/\nunit\n/output\n/sig\nparams/\nstdio\ncapability/\nstdio\n",
            "/capability\n/params\nprint/\nstdio\nstring-literal/\nshould-not-run\n",
            "/string-literal\n/print\n/main\n"
        ),
    )
    .expect("write capability fixture source");
    let contracts = lkjscript_contracts::current_contracts().expect("load current contracts");
    let manifest_contract = contracts
        .get(lkjscript_contracts::PACKAGE_MANIFEST)
        .expect("package manifest contract")
        .digest()
        .to_hex();
    std::fs::write(
        root.join(lkjscript_compiler::package::MANIFEST_FILE),
        format!(
            concat!(
                "{{\n  \"schema\": \"lkjscript.package\",\n",
                "  \"contract\": \"{}\",\n  \"name\": \"denial\",\n",
                "  \"source_root\": \".\",\n  \"modules\": [\"main.lkjscript\"],\n",
                "  \"public\": [\"main.lkjscript\"],\n  \"dependencies\": [],\n",
                "  \"capabilities\": [],\n  \"targets\": [{{\"name\": \"main\", ",
                "\"module\": \"main.lkjscript\"}}]\n}}\n"
            ),
            manifest_contract
        ),
    )
    .expect("write capability fixture manifest");
    let (lock_path, lock) =
        lkjscript_compiler::package::create_lock(&entry).expect("create capability fixture lock");
    std::fs::write(lock_path, lock).expect("write capability fixture lock");

    let output = Command::new(binary)
        .args([
            "run",
            entry.to_str().expect("UTF-8 capability fixture path"),
        ])
        .output()
        .expect("run denied capability fixture");
    std::fs::remove_dir_all(&root).expect("remove capability fixture");

    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "denied program performed stdout effect"
    );
    let stderr = String::from_utf8(output.stderr).expect("denial diagnostic is UTF-8");
    assert!(
        stderr.contains("package does not grant required stdio capability"),
        "{stderr}"
    );
}

#[test]
fn deleted_platform_commands_are_rejected() {
    let binary = env!("CARGO_BIN_EXE_lkjscript");
    for command in ["runtime", "semantic", "system"] {
        let output = Command::new(binary)
            .arg(command)
            .output()
            .expect("run deleted command");
        assert!(
            !output.status.success(),
            "deleted command accepted: {command}"
        );
        assert!(String::from_utf8(output.stderr)
            .expect("diagnostic is UTF-8")
            .contains(&format!("unknown command: {command}")));
    }

    let version = Command::new(binary)
        .arg("--version")
        .output()
        .expect("run version command");
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).expect("version is UTF-8"),
        format!("lkjscript {}\n", env!("CARGO_PKG_VERSION"))
    );
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
    assert!(text.contains("current exact test-oracle/VM/preferred-baseline byte-vector subset"));
}
