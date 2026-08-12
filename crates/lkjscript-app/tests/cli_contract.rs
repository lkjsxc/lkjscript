#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::process::Command;

const CHECK_USAGE: &str = "usage: lkjscript check <file.lkjscript> [--json]";

fn read_metrics(path: &std::path::Path) -> serde_json::Value {
    let line = std::fs::read_to_string(path).expect("read metrics output");
    let json = line
        .strip_prefix("LKJSCRIPT_METRICS ")
        .expect("metrics marker");
    serde_json::from_str(json).expect("metrics are valid JSON")
}

#[test]
fn help_and_metrics_expose_current_product_truth() {
    let binary = env!("CARGO_BIN_EXE_lkjscript");
    let help = Command::new(binary)
        .arg("--help")
        .output()
        .expect("run CLI help");
    assert!(help.status.success());
    assert!(help.stderr.is_empty());
    let help = String::from_utf8(help.stdout).expect("help is UTF-8");
    assert!(help.contains("check <file.lkjscript> [--json]"));
    assert!(help.contains("compile a required package without entering the program"));
    assert!(help.contains("run <file.lkjscript> [--] [script-args...]"));
    assert!(help.contains("compile and intentionally execute the program"));
    assert!(help.contains("memory inventory [--json]"));
    assert!(!help.contains("describe"));
    assert!(!help.contains("line-oriented language"));
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
    assert_eq!(json["schema"].as_str(), Some("lkjscript.metrics"));
    assert_eq!(
        json["contract"].as_str(),
        Some(lkjscript_contracts::METRICS_DIGEST.to_hex().as_str())
    );
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
fn check_is_quiet_deterministic_and_does_not_enter_effectful_programs() {
    let binary = env!("CARGO_BIN_EXE_lkjscript");
    let hello =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/examples/hello/main.lkjscript");

    for metrics_enabled in [false, true] {
        let mut command = Command::new(binary);
        command.arg("check").arg(&hello);
        if metrics_enabled {
            command.env("LKJSCRIPT_METRICS", "1");
        }
        let checked = command.output().expect("check effectful hello program");
        assert!(checked.status.success(), "stderr={:?}", checked.stderr);
        assert!(checked.stdout.is_empty());
        assert!(checked.stderr.is_empty());
    }

    let first = Command::new(binary)
        .arg("check")
        .arg(&hello)
        .arg("--json")
        .output()
        .expect("check hello as JSON");
    let second = Command::new(binary)
        .arg("check")
        .arg(&hello)
        .arg("--json")
        .output()
        .expect("repeat JSON check");
    assert!(first.status.success());
    assert!(first.stderr.is_empty());
    assert_eq!(first.status, second.status);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    let document: serde_json::Value =
        serde_json::from_slice(&first.stdout).expect("check success is JSON");
    assert_eq!(
        document,
        serde_json::json!({"schema": "lkjscript.check", "status": "ok"})
    );

    let run = Command::new(binary)
        .arg("run")
        .arg(&hello)
        .output()
        .expect("run effectful hello program");
    assert!(run.status.success(), "stderr={:?}", run.stderr);
    assert_eq!(run.stdout, b"3628800");
    assert!(run.stderr.is_empty());

    let runtime_failure =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/checked-failure.lkjscript");
    let checked = Command::new(binary)
        .arg("check")
        .arg(&runtime_failure)
        .output()
        .expect("check runtime-failing program");
    assert!(checked.status.success(), "stderr={:?}", checked.stderr);
    assert!(checked.stdout.is_empty());
    assert!(checked.stderr.is_empty());
    let run = Command::new(binary)
        .arg("run")
        .arg(&runtime_failure)
        .output()
        .expect("run runtime-failing program");
    assert!(!run.status.success());
    assert!(run.stdout.is_empty());
    assert!(String::from_utf8(run.stderr)
        .expect("runtime failure is UTF-8")
        .contains("division by zero"));
}

#[test]
fn check_preserves_source_diagnostics_in_human_and_machine_results() {
    let binary = env!("CARGO_BIN_EXE_lkjscript");
    let root = std::env::temp_dir().join(format!(
        "lkjscript-cli-invalid-source-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("cli")
    ));
    std::fs::create_dir_all(&root).expect("create invalid source fixture");
    let entry = root.join("main.lkjscript");
    std::fs::write(
        &entry,
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n",
    )
    .expect("write initially valid source");
    let manifest_contract = lkjscript_contracts::PACKAGE_MANIFEST_DIGEST.to_hex();
    std::fs::write(
        root.join(lkjscript_compiler::package::MANIFEST_FILE),
        format!(
            concat!(
                "{{\n  \"schema\": \"lkjscript.package\",\n",
                "  \"contract\": \"{}\",\n  \"name\": \"invalid-source\",\n",
                "  \"source_root\": \".\",\n  \"modules\": [\"main.lkjscript\"],\n",
                "  \"public\": [\"main.lkjscript\"],\n  \"dependencies\": [],\n",
                "  \"capabilities\": [],\n  \"targets\": [{{\"name\": \"main\", ",
                "\"module\": \"main.lkjscript\"}}]\n}}\n"
            ),
            manifest_contract
        ),
    )
    .expect("write invalid source fixture manifest");
    let (lock_path, lock) =
        lkjscript_compiler::package::create_lock(&entry).expect("create fixture lock");
    std::fs::write(lock_path, lock).expect("write fixture lock");
    std::fs::write(&entry, "main/\n/wrong\n").expect("make source malformed");

    let human = Command::new(binary)
        .arg("check")
        .arg(&entry)
        .output()
        .expect("check malformed source");
    assert!(!human.status.success());
    assert!(human.stdout.is_empty());
    assert_eq!(
        String::from_utf8(human.stderr).expect("human diagnostic is UTF-8"),
        concat!(
            "main.lkjscript:2:1: error[LKJ-SRC-UNMATCHED-MARKER]: ",
            "mismatched close marker /wrong; expected /main\n",
            "  related main.lkjscript:1:1: opening marker main/\n"
        )
    );

    let first = Command::new(binary)
        .arg("check")
        .arg(&entry)
        .arg("--json")
        .output()
        .expect("check malformed source as JSON");
    let second = Command::new(binary)
        .arg("check")
        .arg(&entry)
        .arg("--json")
        .output()
        .expect("repeat malformed source JSON check");
    std::fs::remove_dir_all(&root).expect("remove invalid source fixture");

    assert!(!first.status.success());
    assert!(first.stderr.is_empty());
    assert_eq!(first.status, second.status);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    let document: serde_json::Value =
        serde_json::from_slice(&first.stdout).expect("source failure is JSON");
    assert_eq!(
        document,
        serde_json::json!({
            "schema": "lkjscript.check",
            "status": "error",
            "failure": {
                "phase": "source",
                "code": "LKJ-SRC-UNMATCHED-MARKER",
                "severity": "error",
                "category": "source-syntax",
                "message": "mismatched close marker /wrong; expected /main",
                "path": "main.lkjscript",
                "range": {
                    "start": {"line": 2, "column": 1},
                    "end": {"line": 2, "column": 7}
                },
                "related": [{
                    "message": "opening marker main/",
                    "path": "main.lkjscript",
                    "range": {
                        "start": {"line": 1, "column": 1},
                        "end": {"line": 1, "column": 6}
                    }
                }]
            }
        })
    );
    assert!(document["failure"].get("class").is_none());
    assert!(document["failure"].get("identity").is_none());
}

#[test]
fn check_reports_package_and_usage_failures_without_fabricated_source_facts() {
    let binary = env!("CARGO_BIN_EXE_lkjscript");
    let locationless = Command::new(binary)
        .args(["check", "not-source.txt", "--json"])
        .output()
        .expect("check a locationless source failure");
    assert_eq!(locationless.status.code(), Some(1));
    assert!(locationless.stderr.is_empty());
    let locationless: serde_json::Value =
        serde_json::from_slice(&locationless.stdout).expect("locationless failure is JSON");
    assert_eq!(locationless["failure"]["phase"].as_str(), Some("source"));
    assert_eq!(
        locationless["failure"]["code"].as_str(),
        Some("LKJ-SRC-LOAD")
    );
    assert_eq!(
        locationless["failure"]["category"].as_str(),
        Some("source-loading")
    );
    assert!(locationless["failure"].get("path").is_none());
    assert!(locationless["failure"].get("range").is_none());

    let missing_parent = std::env::temp_dir().join(format!(
        "lkjscript-cli-missing-parent-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("cli")
    ));
    let host = Command::new(binary)
        .arg("check")
        .arg(missing_parent.join("main.lkjscript"))
        .arg("--json")
        .output()
        .expect("check missing package parent");
    assert_eq!(host.status.code(), Some(1));
    assert!(host.stderr.is_empty());
    let host: serde_json::Value =
        serde_json::from_slice(&host.stdout).expect("host failure is JSON");
    assert_eq!(host["failure"]["phase"].as_str(), Some("package"));
    assert_eq!(host["failure"]["class"].as_str(), Some("host"));
    assert!(host["failure"].get("path").is_none());

    let undeclared =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/residual-compare.lkjscript");
    let package = Command::new(binary)
        .arg("check")
        .arg(&undeclared)
        .arg("--json")
        .output()
        .expect("check undeclared package module");
    assert!(!package.status.success());
    assert!(package.stderr.is_empty());
    let package: serde_json::Value =
        serde_json::from_slice(&package.stdout).expect("package failure is JSON");
    assert_eq!(package["schema"].as_str(), Some("lkjscript.check"));
    assert_eq!(package["status"].as_str(), Some("error"));
    assert_eq!(package["failure"]["phase"].as_str(), Some("package"));
    assert_eq!(package["failure"]["class"].as_str(), Some("error"));
    assert!(package["failure"]["message"]
        .as_str()
        .is_some_and(|message| message.contains("entry module is not declared")));
    for absent in ["code", "path", "range", "related"] {
        assert!(package["failure"].get(absent).is_none());
    }

    let usage = format!("lkjscript: {CHECK_USAGE}\n");
    for (arguments, expected) in [
        (vec!["check"], usage.as_str()),
        (vec!["check", "--json"], usage.as_str()),
        (
            vec!["check", "main.lkjscript", "script-argument"],
            usage.as_str(),
        ),
        (
            vec!["check", "main.lkjscript", "--json", "extra"],
            usage.as_str(),
        ),
        (
            vec!["check", "main.lkjscript", "--unknown"],
            "lkjscript: unknown check option: --unknown\n",
        ),
    ] {
        let output = Command::new(binary)
            .args(&arguments)
            .output()
            .expect("run invalid check usage");
        assert_eq!(output.status.code(), Some(1), "accepted {arguments:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).expect("usage failure is UTF-8"),
            expected
        );
    }
}

#[test]
fn package_check_success_is_quiet() {
    let binary = env!("CARGO_BIN_EXE_lkjscript");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(binary)
        .args(["package", "check"])
        .arg(root)
        .output()
        .expect("check repository package");
    assert!(output.status.success(), "stderr={:?}", output.stderr);
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
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
    let manifest_contract = lkjscript_contracts::PACKAGE_MANIFEST_DIGEST.to_hex();
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

    let checked = Command::new(binary)
        .arg("check")
        .arg(&entry)
        .output()
        .expect("check denied capability fixture");
    assert!(!checked.status.success());
    assert!(
        checked.stdout.is_empty(),
        "check performed the denied stdout effect"
    );
    let check_stderr = String::from_utf8(checked.stderr).expect("check denial diagnostic is UTF-8");
    assert!(
        check_stderr.contains("package does not grant required stdio capability"),
        "{check_stderr}"
    );
    assert!(!check_stderr.contains("should-not-run"));

    let machine = Command::new(binary)
        .arg("check")
        .arg(&entry)
        .arg("--json")
        .output()
        .expect("check denied capability fixture as JSON");
    assert_eq!(machine.status.code(), Some(1));
    assert!(machine.stderr.is_empty());
    let machine: serde_json::Value =
        serde_json::from_slice(&machine.stdout).expect("capability denial is JSON");
    assert_eq!(machine["failure"]["phase"].as_str(), Some("package"));
    assert_eq!(machine["failure"]["class"].as_str(), Some("error"));
    assert_eq!(
        machine["failure"]["message"].as_str(),
        Some("package does not grant required stdio capability")
    );
    assert!(machine["failure"].get("path").is_none());

    let run = Command::new(binary)
        .arg("run")
        .arg(&entry)
        .output()
        .expect("run denied capability fixture");
    std::fs::remove_dir_all(&root).expect("remove capability fixture");

    assert!(!run.status.success());
    assert!(
        run.stdout.is_empty(),
        "denied program performed stdout effect"
    );
    let run_stderr = String::from_utf8(run.stderr).expect("denial diagnostic is UTF-8");
    assert!(
        run_stderr.contains("package does not grant required stdio capability"),
        "{run_stderr}"
    );
}

#[test]
fn deleted_commands_are_rejected() {
    let binary = env!("CARGO_BIN_EXE_lkjscript");
    for command in ["describe", "runtime", "semantic", "system"] {
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
    let repeated = Command::new(binary)
        .args(["memory", "inventory", "--json"])
        .output()
        .expect("repeat memory inventory");
    assert!(repeated.status.success());
    assert!(repeated.stderr.is_empty());
    assert_eq!(inventory.stdout, repeated.stdout);

    let document: serde_json::Value =
        serde_json::from_slice(&inventory.stdout).expect("inventory is JSON");
    assert_eq!(
        document["schema"].as_str(),
        Some(lkjscript_contracts::MEMORY_OBLIGATIONS)
    );
    assert_eq!(
        document["contract"].as_str(),
        Some(
            lkjscript_contracts::MEMORY_OBLIGATIONS_DIGEST
                .to_hex()
                .as_str()
        )
    );
    let entries = document["entries"]
        .as_array()
        .expect("inventory entries are an array");
    let identities = entries
        .iter()
        .map(|entry| entry["identity"].as_str().expect("entry identity"))
        .collect::<Vec<_>>();
    let expected = lkjscript_contracts::memory_obligations()
        .iter()
        .map(|record| record.identity)
        .collect::<Vec<_>>();
    assert_eq!(identities, expected);
    let enumeration = entries
        .iter()
        .find(|entry| entry["identity"] == "enum")
        .expect("enum inventory entry");
    assert_eq!(enumeration["current_trace_fields"], "none");
    assert!(entries.iter().any(|entry| {
        entry["status"].as_str().is_some_and(|value| {
            value.contains("current deterministic storage; unsupported shapes reject")
        })
    }));
    assert!(entries.iter().any(|entry| {
        entry.as_object().is_some_and(|entry| {
            entry.values().any(|value| {
                value.as_str().is_some_and(|value| {
                    value.contains("verified static image data or execution-owned unique store")
                })
            })
        })
    }));

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
