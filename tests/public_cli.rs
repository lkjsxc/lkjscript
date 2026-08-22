#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "the black-box test harness uses panic-on-failure assertions"
)]

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const APPLICATION: &str = "applications/lkjournal";
const CLI_CONTRACT_VERSION: u64 = 4;

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lkjscript"))
}

fn command(arguments: &[&str]) -> Output {
    command_at(&binary(), Path::new(env!("CARGO_MANIFEST_DIR")), arguments)
}

fn command_at(executable: &Path, directory: &Path, arguments: &[&str]) -> Output {
    let context = format!(
        "run public CLI '{}' with {arguments:?}",
        executable.display()
    );
    Command::new(executable)
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect(&context)
}

fn success(arguments: &[&str]) -> Value {
    success_output(command(arguments))
}

fn success_at(executable: &Path, directory: &Path, arguments: &[&str]) -> Value {
    success_output(command_at(executable, directory, arguments))
}

fn success_output(output: Output) -> Value {
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
    assert_eq!(value["contract_version"], CLI_CONTRACT_VERSION);
    value
}

fn failure(arguments: &[&str]) -> Value {
    let output = command(arguments);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    assert!(output.stdout.len() < 16 * 1024);
    let value: Value = serde_json::from_slice(&output.stdout).expect("failure JSON");
    assert_eq!(value["ok"], false);
    assert_eq!(value["contract_version"], CLI_CONTRACT_VERSION);
    value
}

#[test]
fn direct_cli_discovery_query_check_build_and_inspection_are_compact() {
    let capabilities = success(&["capabilities"]);
    assert_eq!(
        capabilities["result"]["commands"],
        serde_json::json!([
            "capabilities",
            "new",
            "inspect",
            "query",
            "change",
            "draft",
            "history",
            "package",
            "check",
            "build",
            "run",
            "serve",
            "worker",
            "review",
            "backup",
            "restore",
            "doctor"
        ])
    );
    let schema = capabilities["result"]["schema_digest"]
        .as_str()
        .expect("schema digest");
    let cached = success(&["capabilities", "--known-schema", schema]);
    assert_eq!(cached["result"]["unchanged"], true);
    assert!(
        capabilities["result"]["type_forms"]
            .as_array()
            .expect("type forms")
            .contains(&serde_json::json!("parameter"))
    );
    assert!(
        capabilities["result"]["owner_kinds"]
            .as_array()
            .expect("owner kinds")
            .contains(&serde_json::json!("type_parameter"))
    );
    assert!(
        capabilities["result"]["expression_forms"]
            .as_array()
            .expect("expression forms")
            .contains(&serde_json::json!("invoke"))
    );
    assert!(
        capabilities["result"]["expression_forms"]
            .as_array()
            .expect("expression forms")
            .contains(&serde_json::json!("constant"))
    );
    assert_eq!(
        capabilities["result"]["declaration_reference_forms"],
        serde_json::json!([
            "request_local_symbol",
            "local_declaration_id",
            "exact_package_module_declaration"
        ])
    );
    assert_eq!(
        capabilities["result"]["declaration_reference_syntax"]["exact_package_module_declaration"],
        "exact:PACKAGE_HEX/mod_HEX/decl_HEX"
    );
    let change_help = success(&["capabilities", "change"]);
    assert_eq!(change_help["result"]["name"], "change");
    assert!(
        change_help["result"]["usage"]
            .as_str()
            .expect("usage")
            .contains("--commit")
    );

    let project = success(&[
        "--project",
        APPLICATION,
        "inspect",
        "project",
        "--limit",
        "10",
    ]);
    assert_eq!(project["result"]["authority"], "typed_semantic_graph");
    assert_eq!(project["result"]["module_count"], 3);
    assert_eq!(project["result"]["target_count"], 2);
    assert!(
        project["result"]["expansion_commands"]
            .as_array()
            .expect("expansion commands")
            .iter()
            .all(|command| !command.as_str().unwrap().contains(" semantic "))
    );

    let targets = success(&[
        "--project",
        APPLICATION,
        "inspect",
        "targets",
        "--limit",
        "5",
    ]);
    assert_eq!(targets["result"]["items"].as_array().unwrap().len(), 2);

    let found = success(&[
        "--project",
        APPLICATION,
        "query",
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
        "inspect",
        "owner",
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

    let tests = success(&["--project", APPLICATION, "check"]);
    assert_eq!(tests["result"]["passed"], 12);
    assert_eq!(tests["result"]["differential"], "equal");

    let temporary = tempfile::TempDir::new().expect("temporary output");
    let artifact = temporary.path().join("application.lkja");
    let artifact_text = path(&artifact);
    let first = success(&["--project", APPLICATION, "build", "--output", artifact_text]);
    assert_eq!(first["result"]["publication"], "published");
    let repeated = success(&["--project", APPLICATION, "build", "--output", artifact_text]);
    assert_eq!(repeated["result"]["publication"], "unchanged");
    let inspection = success(&["inspect", "artifact", artifact_text]);
    assert_eq!(inspection["result"]["targets"].as_array().unwrap().len(), 2);
}

#[test]
fn copied_binary_creates_runs_backs_up_and_restores_a_command_project() {
    let temporary = tempfile::TempDir::new().expect("isolated binary workspace");
    let copied_binary = temporary.path().join("lkjscript");
    std::fs::copy(binary(), &copied_binary).expect("copy binary");
    let project = temporary.path().join("app");
    let created = success_at(
        &copied_binary,
        temporary.path(),
        &[
            "new",
            path(&project),
            "--template",
            "command",
            "--name",
            "sample",
        ],
    );
    assert_eq!(created["result"]["template"], "command");
    assert!(
        created["result"]["repository_id"]
            .as_str()
            .unwrap()
            .starts_with("repo_")
    );
    assert!(
        created["result"]["builtin_dependency"]["artifact"]
            .as_str()
            .unwrap()
            .starts_with("artifact_")
    );
    assert_eq!(
        created["result"]["allocated_identities"]
            .as_object()
            .unwrap()
            .len(),
        5
    );

    let status = success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "inspect", "status"],
    );
    assert_eq!(status["result"]["revision"], created["result"]["revision"]);
    let checked = success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "check"],
    );
    assert_eq!(checked["result"]["passed"], 8);
    let ran = success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "run", "main"],
    );
    assert_eq!(ran["result"]["result"], "hello");
    assert_eq!(ran["result"]["differential"], "equal");

    let found = success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "main",
            "--exact",
            "--limit",
            "16",
        ],
    );
    let main = found["result"]["items"]
        .as_array()
        .expect("main candidates")
        .iter()
        .find(|item| item["kind"] == "pure_function")
        .and_then(|item| item["id"].as_str())
        .expect("main function");
    let before_edit = success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "inspect", "status"],
    );
    let body_change = serde_json::json!({
        "contract_version": 3,
        "base_revision": before_edit["result"]["revision"],
        "changes": [{
            "change": "replace_body",
            "function": main,
            "body": {"text": "goodbye"}
        }]
    });
    let body_change_path = temporary.path().join("body-change.json");
    std::fs::write(
        &body_change_path,
        serde_json::to_vec(&body_change).expect("body change JSON"),
    )
    .expect("body change request");
    let changed = success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "--request-file",
            path(&body_change_path),
            "--commit",
        ],
    );
    assert_eq!(
        changed["result"]["receipt"]["validation"]["profile"],
        "incremental_pure_body_slice"
    );
    assert_eq!(
        changed["result"]["receipt"]["validation"]["modules_checked"],
        1
    );
    let reran = success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "run", "main"],
    );
    assert_eq!(reran["result"]["result"], "goodbye");
    assert_eq!(reran["result"]["differential"], "equal");

    let artifact = temporary.path().join("sample.lkja");
    let built = success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "build",
            "--output",
            path(&artifact),
        ],
    );
    let backup = temporary.path().join("sample.lkjb");
    let backed_up = success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "backup",
            "--output",
            path(&backup),
        ],
    );
    let restored = temporary.path().join("restored");
    std::fs::create_dir(&restored).expect("empty restore destination");
    let restore = success_at(
        &copied_binary,
        temporary.path(),
        &[
            "restore",
            "--backup",
            path(&backup),
            "--output",
            path(&restored),
        ],
    );
    assert_eq!(
        restore["result"]["receipt"]["revision"],
        backed_up["result"]["receipt"]["revision"]
    );
    let doctor = success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&restored), "doctor", "--deep"],
    );
    assert_eq!(doctor["result"]["valid"], true);
    let restored_artifact = temporary.path().join("restored.lkja");
    let rebuilt = success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&restored),
            "build",
            "--output",
            path(&restored_artifact),
        ],
    );
    assert_eq!(
        rebuilt["result"]["receipt"]["artifact_digest"],
        built["result"]["receipt"]["artifact_digest"]
    );
}

#[test]
fn one_public_change_allocates_a_connected_subgraph_and_reuses_dry_run_lowering() {
    let temporary = tempfile::TempDir::new().expect("temporary graph authority");
    let project = temporary.path().join("project");
    success(&["new", path(&project), "--template", "minimal"]);
    let before = success(&["--project", path(&project), "inspect", "status"]);
    let request = serde_json::json!({
        "contract_version": 3,
        "changes": [
            {"change": "create_module", "as": "$domain", "name": "domain"},
            {
                "change": "create_record",
                "as": "$message",
                "module": "$domain",
                "name": "Message",
                "fields": [{"as": "$message-text", "name": "text", "type": {"type": "text"}}],
                "exported": true
            },
            {
                "change": "create_function",
                "as": "$main",
                "module": "$domain",
                "name": "main",
                "result": {"type": "text"},
                "body": {"text": "hello"},
                "exported": true
            },
            {
                "change": "create_component",
                "as": "$app",
                "module": "$domain",
                "name": "App",
                "ports": [{
                    "as": "$main-port",
                    "name": "main",
                    "result": {"type": "text"},
                    "function": "$main"
                }],
                "exported": true
            },
            {
                "change": "create_test",
                "as": "$main-test",
                "module": "$domain",
                "name": "main_returns_hello",
                "actual": {"call": "$main"},
                "expected": {"text": "hello"}
            },
            {
                "change": "create_target",
                "as": "$main-target",
                "name": "main",
                "component": "$app",
                "port": "$main-port",
                "runner": "command"
            }
        ],
        "intent": "public local-symbol acceptance"
    });
    let request_path = temporary.path().join("change.json");
    std::fs::write(&request_path, serde_json::to_vec(&request).unwrap()).unwrap();

    let dry_run = success(&[
        "--project",
        path(&project),
        "change",
        "--request-file",
        path(&request_path),
        "--dry-run",
    ]);
    assert_eq!(dry_run["status"], "validated");
    let after_dry_run = success(&["--project", path(&project), "inspect", "status"]);
    assert_eq!(
        after_dry_run["result"]["revision"],
        before["result"]["revision"]
    );

    let committed = success(&[
        "--project",
        path(&project),
        "change",
        "--request-file",
        path(&request_path),
        "--commit",
    ]);
    assert_eq!(committed["status"], "accepted_change");
    assert_eq!(
        committed["result"]["allocated_identities"],
        dry_run["result"]["allocated_identities"]
    );
    assert_eq!(
        committed["result"]["allocated_identities"]
            .as_object()
            .unwrap()
            .len(),
        8
    );
    assert!(
        committed["result"]["receipt"]["expansion"]
            .as_str()
            .unwrap()
            .starts_with("history show rev_")
    );
    let checked = success(&["--project", path(&project), "check"]);
    assert_eq!(checked["result"]["passed"], 1);
    let ran = success(&["--project", path(&project), "run", "main"]);
    assert_eq!(ran["result"]["result"], "hello");
}

#[test]
fn broad_change_results_are_bounded_and_expandable() {
    let temporary = tempfile::TempDir::new().expect("temporary graph authority");
    let project = temporary.path().join("project");
    success(&["new", path(&project), "--template", "minimal"]);
    let changes = (1..=100)
        .map(|ordinal| {
            serde_json::json!({
                "change": "create_module",
                "as": format!("$module-{ordinal}"),
                "name": format!("bounded.module{ordinal:03}"),
            })
        })
        .collect::<Vec<_>>();
    let request = serde_json::json!({"contract_version": 3, "changes": changes});
    let request_path = temporary.path().join("change.json");
    std::fs::write(&request_path, serde_json::to_vec(&request).unwrap()).unwrap();
    let applied = success(&[
        "--project",
        path(&project),
        "change",
        "--request-file",
        path(&request_path),
        "--commit",
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
    assert_eq!(
        applied["result"]["receipt"]["validation"]["profile"],
        "incremental_independent_module_create"
    );
    assert_eq!(
        applied["result"]["receipt"]["validation"]["modules_checked"],
        100
    );
    let doctor = success(&["--project", path(&project), "doctor", "--deep"]);
    assert_eq!(doctor["result"]["valid"], true);
}

#[test]
fn removed_commands_and_predecessor_source_authority_reject_exactly() {
    for removed in [
        "semantic",
        "help",
        "id-allocate",
        "import",
        "text-project",
        "export-text",
        "export-bundle",
        "hash",
        "deployment",
    ] {
        let result = failure(&[removed]);
        assert_eq!(result["error"]["code"], "cli_usage", "{removed}");
    }

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
    let output = command(&["--project", path(temporary.path()), "inspect", "project"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).expect("failure JSON");
    assert_eq!(
        value["error"]["code"],
        "semantic_predecessor_source_rejected"
    );
}

#[test]
fn bootstrap_rejects_conflicts_and_builtin_bytes_reproduce_maintained_authority() {
    let temporary = tempfile::TempDir::new().expect("temporary bootstrap parent");
    let project = temporary.path().join("app");
    std::fs::create_dir(&project).expect("destination");
    std::fs::write(project.join("owned.txt"), b"preserve\n").expect("owned file");
    let rejected = command(&["new", path(&project), "--template", "minimal"]);
    assert_eq!(rejected.status.code(), Some(2));
    assert_eq!(
        std::fs::read(project.join("owned.txt")).unwrap(),
        b"preserve\n"
    );

    let first = temporary.path().join("first");
    let second = temporary.path().join("second");
    let first_receipt = success(&["new", path(&first), "--template", "minimal"]);
    let second_receipt = success(&["new", path(&second), "--template", "minimal"]);
    assert_ne!(
        first_receipt["result"]["repository_id"],
        second_receipt["result"]["repository_id"]
    );
    assert_ne!(
        first_receipt["result"]["package_id"],
        second_receipt["result"]["package_id"]
    );
    let repeated = command(&["new", path(&first), "--template", "minimal"]);
    assert_eq!(repeated.status.code(), Some(2));

    let builtin = success(&["package", "builtin", "inspect"]);
    let exported = temporary.path().join("standard.lkja");
    let export = success(&["package", "builtin", "export", "--output", path(&exported)]);
    assert_eq!(export["result"]["package"], builtin["result"]);
    assert_eq!(
        std::fs::read(exported).expect("exported built-in"),
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("applications/lkjournal/dependencies/standard.lkja")
        )
        .expect("maintained standard artifact")
    );
}

#[test]
fn exact_dependency_stage_and_change_use_the_public_protocol() {
    let temporary = tempfile::TempDir::new().expect("temporary dependency workflow");
    let project = temporary.path().join("project");
    success(&["new", path(&project), "--template", "minimal"]);
    let status = success(&["--project", path(&project), "inspect", "status"]);
    let builtin = success(&["package", "builtin", "inspect"]);
    let artifact = temporary.path().join("standard.lkja");
    success(&["package", "builtin", "export", "--output", path(&artifact)]);
    let staged = success(&[
        "--project",
        path(&project),
        "package",
        "stage",
        path(&artifact),
    ]);
    assert_eq!(staged["result"]["status"], "staged");

    let request = serde_json::json!({
        "contract_version": 3,
        "base_revision": status["result"]["revision"],
        "idempotency_key": "public-dependency-add-v1",
        "changes": [{
            "change": "add_dependency",
            "alias": "std",
            "package_id": builtin["result"]["package_id"],
            "semantic_revision": builtin["result"]["semantic_revision"],
            "artifact": builtin["result"]["artifact"]
        }]
    });
    let request_path = temporary.path().join("dependency-change.json");
    std::fs::write(&request_path, serde_json::to_vec(&request).unwrap()).unwrap();
    let committed = success(&[
        "--project",
        path(&project),
        "change",
        "--request-file",
        path(&request_path),
        "--commit",
    ]);
    assert_eq!(committed["status"], "accepted_change");
    let project = success(&["--project", path(&project), "inspect", "project"]);
    assert_eq!(project["result"]["dependency_count"], 1);
    assert_eq!(project["result"]["dependencies"][0]["alias"], "std");
    assert_eq!(
        project["result"]["dependencies"][0]["artifact"],
        builtin["result"]["artifact"]
    );
}

fn path(value: &Path) -> &str {
    value.to_str().expect("UTF-8 temporary path")
}
