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
const CLI_CONTRACT_VERSION: u64 = 5;

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
    assert!(matches!(
        compact_field(&records[0], "status"),
        Some("success" | "prepared" | "accepted" | "already-accepted")
    ));
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
            "status",
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
        record.operation == "owner.kind" && compact_field(record, "name") == Some("module")
    }));
    assert!(!owner_section.iter().any(|record| {
        record.operation == "owner.kind"
            && matches!(
                compact_field(record, "name"),
                Some("field" | "expression" | "documentation" | "annotation")
            )
    }));

    let expression_section = compact_success(&["capabilities", "--section", "expression"]);
    for expected in ["call", "constant"] {
        assert!(expression_section.iter().any(|record| {
            record.operation == "expression.form" && compact_field(record, "name") == Some(expected)
        }));
    }

    let change_section = compact_success(&["capabilities", "--section", "change"]);
    let references = change_section
        .iter()
        .filter(|record| record.operation == "change.reference")
        .collect::<Vec<_>>();
    assert_eq!(references.len(), 5);
    let exact_reference = references
        .iter()
        .find(|record| compact_field(record, "name") == Some("exact_package_declaration"))
        .expect("exact reference form");
    assert_eq!(
        compact_field(exact_reference, "syntax"),
        Some("pkg_HEX/decl_HEX")
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
    let change_usage = compact_field(change_operation, "usage").expect("usage");
    assert!(change_usage.contains("change plan"));
    assert!(change_usage.contains("change apply"));
    assert!(change_usage.contains("--plan DIGEST"));

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
    let status_help = compact_success(&["capabilities", "status"]);
    let status_operation = compact_record(&status_help, "operation");
    assert_eq!(compact_field(status_operation, "usage"), Some("status"));
    assert_eq!(
        compact_field(status_operation, "response-model"),
        Some("status_result")
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
fn direct_cli_query_check_and_build_are_bounded() {
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
    assert_eq!(found["result"]["items"][0]["kind"], "component");

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
    let inspection = compact_failure_output(command(&["inspect", "artifact", artifact_text]));
    assert_eq!(
        compact_field(compact_record(&inspection, "diagnostic"), "code"),
        Some("predecessor_contract")
    );
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

    let status = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "status"],
    ));
    assert_eq!(
        compact_field(compact_record(&status, "diagnostic"), "code"),
        Some("predecessor_contract")
    );
    let inspected = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "inspect",
            "owner",
            "module",
            "mod_00000000000000000000000000000001",
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&inspected, "diagnostic"), "code"),
        Some("predecessor_contract")
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

    let rejected_change = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "--input",
            r#"{"contract_version":3,"changes":[]}"#,
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&rejected_change, "diagnostic"), "code"),
        Some("control_operation")
    );

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
fn public_change_reuses_planned_allocation_and_replaces_an_existing_body() {
    let temporary = tempfile::TempDir::new().expect("temporary graph authority");
    let project = temporary.path().join("project");
    let created = compact_success(&["new", path(&project), "--name", "project"]);
    let revision =
        compact_field(compact_record(&created, "revision"), "id").expect("created revision");
    let request = format!(
        "request base={revision} idempotency=connected-public-1 intent=connected-creation\n\
         create.module as=$domain name=domain\n\
         create.record as=$message module=$domain name=Message visibility=public\n\
         add.field as=$message-text record=$message name=text type=text\n\
         expression.local as=$read value=$value\n\
         expression.sequence as=$body\n\
         expression.argument parent=$body index=0 expression=$read\n\
         create.function as=$main module=$domain name=main visibility=public result=text effect=pure body=$body\n\
         add.parameter as=$value function=$main name=value type=text\n"
    );
    let request_path = temporary.path().join("change.lkjc");
    std::fs::write(&request_path, request).expect("compact change request");

    let planned = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&request_path),
    ]);
    assert_eq!(compact_field(&planned[0], "status"), Some("prepared"));
    assert_eq!(
        compact_field(compact_record(&planned, "revision"), "base"),
        Some(revision)
    );
    let plan = compact_field(compact_record(&planned, "plan"), "digest")
        .expect("reviewed plan")
        .to_owned();
    let planned_identities = planned
        .iter()
        .filter(|record| record.operation == "identity")
        .map(|record| {
            (
                compact_field(record, "symbol").unwrap(),
                compact_field(record, "id").unwrap(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(planned_identities.len(), 7);

    let committed = compact_success(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "--input-file",
        path(&request_path),
        "--plan",
        &plan,
    ]);
    assert_eq!(compact_field(&committed[0], "status"), Some("accepted"));
    assert_eq!(
        committed
            .iter()
            .filter(|record| record.operation == "identity")
            .map(|record| {
                (
                    compact_field(record, "symbol").unwrap(),
                    compact_field(record, "id").unwrap(),
                )
            })
            .collect::<Vec<_>>(),
        planned_identities
    );
    let accepted_revision = compact_field(compact_record(&committed, "revision"), "result")
        .expect("accepted revision")
        .to_owned();
    let status = compact_success(&["--project", path(&project), "status"]);
    assert_eq!(
        compact_field(compact_record(&status, "revision"), "id"),
        Some(accepted_revision.as_str())
    );

    let mismatch = compact_failure_output(command(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "--input-file",
        path(&request_path),
        "--plan",
        "plan_0000000000000000000000000000000000000000000000000000000000000000",
    ]));
    assert_eq!(
        compact_field(compact_record(&mismatch, "diagnostic"), "code"),
        Some("change_plan_mismatch")
    );

    let stale_output = command(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&request_path),
    ]);
    assert_eq!(stale_output.status.code(), Some(7));
    let stale = parse_records("stdout", &stale_output.stdout).expect("compact stale response");
    assert_eq!(
        compact_field(compact_record(&stale, "diagnostic"), "code"),
        Some("change_authored_stale_base")
    );

    let function = planned_identities
        .iter()
        .find_map(|(symbol, identity)| (*symbol == "$main").then_some(*identity))
        .expect("allocated function identity");
    let replacement = format!(
        "request base={accepted_revision}\n\
         expression.text as=$replacement value=replaced\n\
         replace.body function={function} body=$replacement\n"
    );
    let replacement_path = temporary.path().join("replacement.lkjc");
    std::fs::write(&replacement_path, replacement).expect("replacement change request");
    let replacement_plan = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&replacement_path),
    ]);
    assert_eq!(
        compact_field(compact_record(&replacement_plan, "summary"), "updated"),
        Some("1")
    );
    assert_eq!(
        compact_field(compact_record(&replacement_plan, "summary"), "deleted"),
        Some("2")
    );
    let replacement_digest = compact_field(compact_record(&replacement_plan, "plan"), "digest")
        .expect("replacement plan digest");
    let replaced = compact_success(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "--input-file",
        path(&replacement_path),
        "--plan",
        replacement_digest,
    ]);
    assert_eq!(compact_field(&replaced[0], "status"), Some("accepted"));
    assert_eq!(
        compact_field(compact_record(&replaced, "summary"), "updated"),
        Some("1")
    );
}

#[test]
fn broad_change_results_are_bounded_and_expandable() {
    let temporary = tempfile::TempDir::new().expect("temporary graph authority");
    let project = temporary.path().join("project");
    let created = compact_success(&["new", path(&project), "--name", "project"]);
    let revision = compact_field(compact_record(&created, "revision"), "id").unwrap();
    let mut request = format!("request base={revision}\n");
    for ordinal in 1..=100 {
        request.push_str(&format!(
            "create.module as=$module-{ordinal} name=module_{ordinal:03}\n"
        ));
    }
    let request_path = temporary.path().join("change.lkjc");
    std::fs::write(&request_path, request).unwrap();
    let planned = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&request_path),
    ]);
    let plan = compact_field(compact_record(&planned, "plan"), "digest").unwrap();
    assert_eq!(
        planned
            .iter()
            .filter(|record| record.operation == "identity")
            .count(),
        100
    );
    let applied = compact_success(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "--input-file",
        path(&request_path),
        "--plan",
        plan,
    ]);
    assert_eq!(compact_field(&applied[0], "status"), Some("accepted"));
    assert_eq!(
        compact_field(compact_record(&applied, "summary"), "created"),
        Some("100")
    );
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
    let value = compact_failure_output(command(&[
        "--project",
        path(temporary.path()),
        "inspect",
        "project",
    ]));
    assert_eq!(
        compact_field(compact_record(&value, "diagnostic"), "code"),
        Some("predecessor_contract")
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

    let nested = first.join("ordinary/nested");
    std::fs::create_dir_all(&nested).expect("nested project directory");
    let status = compact_success_at(&copied_binary, &nested, &["status"]);
    assert_eq!(compact_field(&status[0], "command"), Some("status"));
    assert_eq!(
        compact_field(compact_record(&status, "repository"), "id"),
        Some(first_repository)
    );
    assert_eq!(
        compact_field(compact_record(&status, "package"), "id"),
        Some(first_package)
    );
    assert_eq!(
        compact_field(compact_record(&status, "summary"), "owners"),
        Some("0")
    );
    assert!(compact_field(compact_record(&status, "schema"), "registry").is_some());
    let explicit_status = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&nested), "status"],
    );
    assert_eq!(
        compact_field(compact_record(&explicit_status, "revision"), "id"),
        compact_field(compact_record(&status, "revision"), "id")
    );
    let removed_status_alias =
        compact_failure_output(command_at(&copied_binary, &nested, &["inspect", "status"]));
    assert_eq!(
        compact_field(compact_record(&removed_status_alias, "diagnostic"), "code"),
        Some("predecessor_contract")
    );
    let removed_project =
        compact_failure_output(command_at(&copied_binary, &nested, &["inspect", "project"]));
    assert_eq!(
        compact_field(compact_record(&removed_project, "diagnostic"), "code"),
        Some("predecessor_contract")
    );
    let unknown_owner = compact_failure_output(command_at(
        &copied_binary,
        &nested,
        &[
            "inspect",
            "owner",
            "module",
            "mod_00000000000000000000000000000001",
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&unknown_owner, "diagnostic"), "code"),
        Some("owner_not_found")
    );
    let wrong_kind = compact_failure_output(command_at(
        &copied_binary,
        &nested,
        &[
            "inspect",
            "owner",
            "record",
            "mod_00000000000000000000000000000001",
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&wrong_kind, "diagnostic"), "code"),
        Some("owner_wrong_kind")
    );
    let malformed = compact_failure_output(command_at(
        &copied_binary,
        &nested,
        &["inspect", "owner", "module", "mod_not-hex"],
    ));
    assert_eq!(
        compact_field(compact_record(&malformed, "diagnostic"), "code"),
        Some("owner_selector_identity")
    );
    let fine_owner = compact_failure_output(command_at(
        &copied_binary,
        &nested,
        &[
            "inspect",
            "owner",
            "field",
            "field_00000000000000000000000000000001",
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&fine_owner, "diagnostic"), "code"),
        Some("owner_selector_kind")
    );
    let predecessor_owner = compact_failure_output(command_at(
        &copied_binary,
        &nested,
        &["inspect", "owner", "mod_00000000000000000000000000000001"],
    ));
    assert_eq!(
        compact_field(compact_record(&predecessor_owner, "diagnostic"), "code"),
        Some("predecessor_contract")
    );
    let foreign_package = if first_package == "pkg_00000000000000000000000000000001" {
        "pkg_00000000000000000000000000000002"
    } else {
        "pkg_00000000000000000000000000000001"
    };
    let foreign = compact_failure_output(command_at(
        &copied_binary,
        &nested,
        &[
            "inspect",
            "owner",
            "module",
            "mod_00000000000000000000000000000001",
            "--package",
            foreign_package,
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&foreign, "diagnostic"), "code"),
        Some("owner_foreign_package")
    );
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
fn predecessor_json_change_is_rejected_without_advancing_normalized_authority() {
    let temporary = tempfile::TempDir::new().expect("temporary rejection workflow");
    let project = temporary.path().join("project");
    let created = compact_success(&["new", path(&project), "--name", "project"]);
    let revision = compact_field(compact_record(&created, "revision"), "id").unwrap();
    let request_path = temporary.path().join("change.json");
    std::fs::write(
        &request_path,
        format!(r#"{{"base":"{revision}","changes":[]}}"#),
    )
    .unwrap();
    let rejected = compact_failure_output(command(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&request_path),
    ]));
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some("control_operation")
    );
    let status = compact_success(&["--project", path(&project), "status"]);
    assert_eq!(
        compact_field(compact_record(&status, "revision"), "id"),
        Some(revision)
    );
}

fn path(value: &Path) -> &str {
    value.to_str().expect("UTF-8 temporary path")
}
