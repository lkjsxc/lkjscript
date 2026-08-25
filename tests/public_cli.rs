#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "the black-box test harness uses panic-on-failure assertions"
)]

use lkjscript::platform::control::{CompactRecord, parse_records};
use lkjscript::platform::{ProjectCreationReceipt, ProjectTemplate, create_project};
use serde_json::Value;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const APPLICATION: &str = "applications/lkjournal";
const CLI_CONTRACT_VERSION: u64 = 4;

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lkjscript"))
}

fn copy_executable(source: &Path, destination: &Path) {
    let stage = destination.with_extension("stage");
    let mut input = File::open(source).expect("open executable for isolated copy");
    let permissions = input
        .metadata()
        .expect("inspect executable permissions")
        .permissions();
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&stage)
        .expect("create private executable stage");
    io::copy(&mut input, &mut output).expect("copy executable into private stage");
    output
        .set_permissions(permissions)
        .expect("preserve executable permissions");
    output.sync_all().expect("synchronize executable stage");
    drop(output);
    drop(input);
    std::fs::rename(stage, destination).expect("publish closed executable copy");
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

fn compact_success(arguments: &[&str]) -> Vec<CompactRecord> {
    compact_success_output(command(arguments))
}

fn compact_success_at(
    executable: &Path,
    directory: &Path,
    arguments: &[&str],
) -> Vec<CompactRecord> {
    compact_success_output(command_at(executable, directory, arguments))
}

fn compact_success_output(output: Output) -> Vec<CompactRecord> {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stderr.is_empty());
    assert!(
        output.stdout.len() < 64 * 1024,
        "compact success output is excessive"
    );
    let records = parse_records("stdout", &output.stdout).expect("compact records");
    assert_eq!(compact_field(&records[0], "status"), Some("success"));
    records
}

fn compact_field<'a>(record: &'a CompactRecord, name: &str) -> Option<&'a str> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.as_str())
}

fn compact_record<'a>(records: &'a [CompactRecord], operation: &str) -> &'a CompactRecord {
    records
        .iter()
        .find(|record| record.operation == operation)
        .expect("compact record")
}

fn compact_failure_output(output: Output) -> Vec<CompactRecord> {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    assert!(output.stdout.len() < 16 * 1024);
    let records = parse_records("stdout", &output.stdout).expect("compact failure records");
    assert_eq!(compact_field(&records[0], "status"), Some("failure"));
    records
}

fn predecessor_project(
    destination: &Path,
    name: &str,
    template: ProjectTemplate,
) -> ProjectCreationReceipt {
    // Test-only setup for public operations that have not crossed to normalized authority yet.
    // Delete this fixture boundary when those operations cut over.
    create_project(destination, name, template).expect("predecessor fixture bootstrap")
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
fn capabilities_discovery_is_compact_focused_and_exportable() {
    let capabilities = compact_success(&["capabilities"]);
    assert_eq!(
        capabilities
            .iter()
            .filter(|record| record.operation == "command")
            .filter_map(|record| compact_field(record, "name"))
            .collect::<Vec<_>>(),
        vec![
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
        ]
    );
    let registry = compact_record(&capabilities, "registry");
    let registry_digest = compact_field(registry, "digest").expect("registry digest");
    let cached = compact_success(&["capabilities", "--known-registry", registry_digest]);
    assert_eq!(compact_field(&cached[0], "unchanged"), Some("true"));
    assert_eq!(cached.len(), 2);
    let implicit = compact_success_output(command(&[]));
    assert_eq!(
        compact_field(compact_record(&implicit, "registry"), "digest"),
        Some(registry_digest)
    );
    let project_scoped = compact_success(&[
        "--project",
        APPLICATION,
        "capabilities",
        "--known-registry",
        registry_digest,
    ]);
    assert_eq!(compact_field(&project_scoped[0], "unchanged"), Some("true"));

    let type_section = compact_success(&["capabilities", "--section", "type"]);
    assert!(type_section.iter().any(|record| {
        record.operation == "type.form" && compact_field(record, "name") == Some("parameter")
    }));

    let owner_section = compact_success(&["capabilities", "--section", "owners"]);
    assert!(owner_section.iter().any(|record| {
        record.operation == "owner.kind" && compact_field(record, "name") == Some("type_parameter")
    }));

    let expression_section = compact_success(&["capabilities", "--section", "expression"]);
    for expected in ["invoke", "constant"] {
        assert!(expression_section.iter().any(|record| {
            record.operation == "expression.form" && compact_field(record, "name") == Some(expected)
        }));
    }

    let change_section = compact_success(&["capabilities", "--section", "change"]);
    let references = change_section
        .iter()
        .filter(|record| record.operation == "change.reference")
        .collect::<Vec<_>>();
    assert_eq!(references.len(), 3);
    let exact_reference = references
        .iter()
        .find(|record| compact_field(record, "name") == Some("exact_package_module_declaration"))
        .expect("exact reference form");
    assert_eq!(
        compact_field(exact_reference, "syntax"),
        Some("exact:PACKAGE_HEX/mod_HEX/decl_HEX")
    );
    let known_type_digest = compact_field(compact_record(&type_section, "section"), "digest")
        .expect("type section digest");
    let known_type = format!("type={known_type_digest}");
    let unchanged_type = compact_success(&["capabilities", "--known-section", &known_type]);
    assert_eq!(compact_field(&unchanged_type[0], "unchanged"), Some("true"));
    assert_eq!(
        compact_field(compact_record(&unchanged_type, "section"), "changed"),
        Some("false")
    );

    let change_help = compact_success(&["capabilities", "change"]);
    let change_operation = compact_record(&change_help, "operation");
    assert_eq!(compact_field(change_operation, "name"), Some("change"));
    assert!(
        compact_field(change_operation, "usage")
            .expect("usage")
            .contains("--commit")
    );

    let new_help = compact_success(&["capabilities", "new"]);
    let new_operation = compact_record(&new_help, "operation");
    assert_eq!(
        compact_field(new_operation, "response-model"),
        Some("new_result")
    );
    assert_eq!(
        compact_field(new_operation, "usage"),
        Some("new DEST [--template minimal] [--name NAME]")
    );
    let templates = compact_success(&["capabilities", "--section", "templates"]);
    assert_eq!(
        templates
            .iter()
            .filter(|record| record.operation == "template")
            .filter_map(|record| compact_field(record, "name"))
            .collect::<Vec<_>>(),
        vec!["minimal"]
    );

    let temporary_capabilities = tempfile::TempDir::new().expect("capabilities output");
    let registry_path = temporary_capabilities.path().join("registry.lkjc");
    let registry_path_text = path(&registry_path);
    let exported = compact_success(&["capabilities", "--output", registry_path_text]);
    let file = compact_record(&exported, "file");
    assert_eq!(compact_field(file, "kind"), Some("registry"));
    assert_eq!(compact_field(file, "digest"), Some(registry_digest));
    let exported_bytes = std::fs::read(&registry_path).expect("compact registry export");
    let exported_records = parse_records("registry.lkjc", &exported_bytes).expect("export records");
    assert!(exported_records.len() > capabilities.len());
    assert_ne!(exported_bytes.first(), Some(&b'{'));

    let verified = compact_success(&["capabilities", "--verify-generated", "docs/generated"]);
    assert_eq!(
        verified
            .iter()
            .filter(|record| record.operation == "file")
            .count(),
        3
    );

    let rejected_schema = command(&["capabilities", "--known-schema", registry_digest]);
    assert_eq!(rejected_schema.status.code(), Some(2));
    assert!(serde_json::from_slice::<Value>(&rejected_schema.stdout).is_err());
    let rejected_records =
        parse_records("stdout", &rejected_schema.stdout).expect("compact failure records");
    assert_eq!(
        compact_field(&rejected_records[0], "status"),
        Some("failure")
    );
    assert_eq!(
        compact_field(compact_record(&rejected_records, "diagnostic"), "code"),
        Some("cli_usage")
    );
}

#[test]
fn direct_cli_query_check_build_and_inspection_are_bounded() {
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
fn copied_binary_rejects_the_predecessor_template_and_runs_a_predecessor_fixture() {
    let temporary = tempfile::TempDir::new().expect("isolated binary workspace");
    let copied_binary = temporary.path().join("lkjscript");
    copy_executable(&binary(), &copied_binary);
    let project = temporary.path().join("app");
    let rejected = compact_failure_output(command_at(
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
    ));
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some("predecessor_contract")
    );
    assert!(!project.exists());

    let created = predecessor_project(&project, "sample", ProjectTemplate::Command);
    assert_eq!(created.template, ProjectTemplate::Command);
    assert!(created.repository_id.to_string().starts_with("repo_"));
    assert!(
        created
            .builtin_dependency
            .as_ref()
            .expect("command template dependency")
            .artifact
            .to_string()
            .starts_with("artifact_")
    );
    assert_eq!(created.allocated_identities.len(), 5);

    let status = success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "inspect", "status"],
    );
    assert_eq!(
        status["result"]["revision"],
        Value::String(created.revision.to_string())
    );
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
    let found_after_body_change = success_at(
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
    assert!(
        found_after_body_change["result"]["items"]
            .as_array()
            .expect("main candidates after body change")
            .iter()
            .any(|item| item["kind"] == "pure_function" && item["id"] == main)
    );

    let rename = serde_json::json!({
        "contract_version": 3,
        "base_revision": changed["result"]["published_revision"],
        "changes": [{
            "change": "rename_declaration",
            "declaration": main,
            "new_name": "entry"
        }]
    });
    let rename_path = temporary.path().join("rename.json");
    std::fs::write(
        &rename_path,
        serde_json::to_vec(&rename).expect("rename JSON"),
    )
    .expect("rename request");
    let renamed = success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "--request-file",
            path(&rename_path),
            "--commit",
        ],
    );
    assert_eq!(
        renamed["result"]["receipt"]["validation"]["profile"],
        "incremental_declaration_rename"
    );
    let found_after_rename = success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "entry",
            "--exact",
            "--limit",
            "16",
        ],
    );
    assert!(
        found_after_rename["result"]["items"]
            .as_array()
            .expect("entry candidates after rename")
            .iter()
            .any(|item| item["kind"] == "pure_function" && item["id"] == main)
    );
    let stale_name = success_at(
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
    assert!(
        stale_name["result"]["items"]
            .as_array()
            .expect("remaining main candidates")
            .iter()
            .all(|item| item["kind"] != "pure_function" || item["id"] != main)
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
    predecessor_project(&project, "project", ProjectTemplate::Minimal);
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
    predecessor_project(&project, "project", ProjectTemplate::Minimal);
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
fn copied_binary_creates_normalized_minimal_projects_and_rejects_unsafe_destinations() {
    let temporary = tempfile::TempDir::new().expect("temporary normalized bootstrap parent");
    let copied_binary = temporary.path().join("lkjscript");
    copy_executable(&binary(), &copied_binary);

    let first = temporary.path().join("first");
    let first_receipt = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&first), "--template", "minimal"],
    );
    assert_eq!(compact_field(&first_receipt[0], "command"), Some("new"));
    assert_eq!(
        compact_field(compact_record(&first_receipt, "project"), "template"),
        Some("minimal")
    );
    assert_eq!(
        compact_field(compact_record(&first_receipt, "project"), "name"),
        Some("first")
    );
    let first_repository = compact_field(compact_record(&first_receipt, "repository"), "id")
        .expect("repository identity");
    let first_package =
        compact_field(compact_record(&first_receipt, "package"), "id").expect("package identity");
    assert!(first_repository.starts_with("repo_"));
    assert!(first_package.starts_with("pkg_"));
    assert!(
        compact_field(compact_record(&first_receipt, "revision"), "id")
            .expect("revision identity")
            .starts_with("rev_")
    );
    assert!(
        compact_field(compact_record(&first_receipt, "state"), "digest")
            .expect("semantic state")
            .starts_with("semantic_state_")
    );
    assert!(
        compact_field(compact_record(&first_receipt, "root"), "digest")
            .expect("semantic root")
            .starts_with("semantic_root_")
    );
    let receipt = compact_record(&first_receipt, "receipt");
    assert!(
        compact_field(receipt, "digest")
            .expect("receipt digest")
            .starts_with("receipt_object_")
    );
    assert!(
        compact_field(receipt, "revision-record")
            .expect("revision record")
            .starts_with("revision_object_")
    );
    assert!(first.join("HEAD").is_file());
    assert!(first.join("LOCK").is_file());
    assert!(!first.join(".lkjscript").exists());

    let second = temporary.path().join("second");
    std::fs::create_dir(&second).expect("existing empty destination");
    let second_receipt = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&second), "--name", "second_project"],
    );
    assert_ne!(
        first_repository,
        compact_field(compact_record(&second_receipt, "repository"), "id").unwrap()
    );
    assert_ne!(
        first_package,
        compact_field(compact_record(&second_receipt, "package"), "id").unwrap()
    );
    assert!(second.join("HEAD").is_file());

    let repeated = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&first)],
    ));
    assert_eq!(
        compact_field(compact_record(&repeated, "diagnostic"), "code"),
        Some("new_destination_not_empty")
    );

    let nonempty = temporary.path().join("nonempty");
    std::fs::create_dir(&nonempty).expect("nonempty destination");
    std::fs::write(nonempty.join("owned.txt"), b"preserve\n").expect("owned file");
    let rejected = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&nonempty)],
    ));
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some("new_destination_not_empty")
    );
    assert_eq!(
        std::fs::read(nonempty.join("owned.txt")).unwrap(),
        b"preserve\n"
    );

    let file = temporary.path().join("destination-file");
    std::fs::write(&file, b"preserve\n").expect("destination file");
    let rejected = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&file)],
    ));
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some("new_destination_type")
    );
    assert_eq!(std::fs::read(&file).unwrap(), b"preserve\n");

    let invalid = temporary.path().join("9invalid");
    let rejected = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&invalid)],
    ));
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some("kernel_name")
    );
    assert!(!invalid.exists());

    let predecessor = temporary.path().join("predecessor");
    std::fs::create_dir(&predecessor).expect("predecessor destination");
    std::fs::create_dir(predecessor.join(".lkjscript")).expect("predecessor marker");
    let rejected = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&predecessor)],
    ));
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some("predecessor_contract")
    );
    assert!(predecessor.join(".lkjscript").is_dir());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let actual_parent = temporary.path().join("actual-parent");
        let linked_parent = temporary.path().join("linked-parent");
        std::fs::create_dir(&actual_parent).expect("actual parent");
        symlink(&actual_parent, &linked_parent).expect("parent symlink");
        let linked_destination = linked_parent.join("project");
        let rejected = compact_failure_output(command_at(
            &copied_binary,
            temporary.path(),
            &["new", path(&linked_destination)],
        ));
        assert_eq!(
            compact_field(compact_record(&rejected, "diagnostic"), "code"),
            Some("new_destination_symlink")
        );
        assert!(!actual_parent.join("project").exists());
    }
}

#[test]
fn builtin_bytes_reproduce_maintained_authority() {
    let temporary = tempfile::TempDir::new().expect("temporary built-in export");
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
    predecessor_project(&project, "project", ProjectTemplate::Minimal);
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
