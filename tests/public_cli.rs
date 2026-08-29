#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "the black-box test harness uses panic-on-failure assertions"
)]

use lkjscript::platform::control::{CompactRecord, decode_logical_change_plan, parse_records};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Read};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;

const APPLICATION: &str = "applications/lkjournal";
const RELEASE_CANDIDATE_ENVIRONMENT: &str = "LKJSCRIPT_RELEASE_CANDIDATE";
const EXECUTABLE_BUSY_ATTEMPTS: usize = 12;
const EXECUTABLE_BUSY_DELAY: Duration = Duration::from_millis(50);

fn binary() -> PathBuf {
    let Some(candidate) = env::var_os(RELEASE_CANDIDATE_ENVIRONMENT) else {
        return PathBuf::from(env!("CARGO_BIN_EXE_lkjscript"));
    };
    let candidate = PathBuf::from(candidate);
    assert!(
        candidate.is_absolute(),
        "{RELEASE_CANDIDATE_ENVIRONMENT} must be an absolute path"
    );
    let metadata =
        std::fs::symlink_metadata(&candidate).expect("inspect explicit release candidate metadata");
    assert!(
        !metadata.file_type().is_symlink() && metadata.is_file(),
        "{RELEASE_CANDIDATE_ENVIRONMENT} must name a regular non-symlink file"
    );
    #[cfg(unix)]
    assert!(
        metadata.permissions().mode() & 0o111 != 0,
        "{RELEASE_CANDIDATE_ENVIRONMENT} must be executable"
    );
    candidate
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
    File::open(destination.parent().expect("copied executable parent"))
        .and_then(|directory| directory.sync_all())
        .expect("synchronize copied executable visibility");
}

fn command(arguments: &[&str]) -> Output {
    command_at(&binary(), Path::new(env!("CARGO_MANIFEST_DIR")), arguments)
}

fn command_at(executable: &Path, directory: &Path, arguments: &[&str]) -> Output {
    let context = format!(
        "run public CLI '{}' with {arguments:?}",
        executable.display()
    );
    retry_executable_busy(|| {
        Command::new(executable)
            .args(arguments)
            .current_dir(directory)
            .env_clear()
            .env("LANG", "C")
            .output()
    })
    .expect(&context)
}

fn retry_executable_busy<T>(mut operation: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut attempt = 1_usize;
    loop {
        match operation() {
            Err(error)
                if error.kind() == io::ErrorKind::ExecutableFileBusy
                    && attempt < EXECUTABLE_BUSY_ATTEMPTS =>
            {
                thread::sleep(EXECUTABLE_BUSY_DELAY);
                attempt += 1;
            }
            result => return result,
        }
    }
}

#[test]
fn copied_binary_spawn_retries_transient_executable_busy() {
    let mut attempts = 0_usize;
    let observed = retry_executable_busy(|| {
        attempts += 1;
        if attempts < 3 {
            Err(io::Error::from(io::ErrorKind::ExecutableFileBusy))
        } else {
            Ok("started")
        }
    })
    .expect("transient executable busy result");
    assert_eq!(observed, "started");
    assert_eq!(attempts, 3);

    let mut exhausted_attempts = 0_usize;
    let error = retry_executable_busy(|| -> io::Result<()> {
        exhausted_attempts += 1;
        Err(io::Error::from(io::ErrorKind::ExecutableFileBusy))
    })
    .expect_err("persistent executable busy result");
    assert_eq!(error.kind(), io::ErrorKind::ExecutableFileBusy);
    assert_eq!(exhausted_attempts, EXECUTABLE_BUSY_ATTEMPTS);
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
    let result = compact_record(&records, "result");
    assert!(matches!(
        compact_field(result, "status"),
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

fn compact_record_values(record: &CompactRecord) -> Vec<(&str, &str)> {
    record
        .fields
        .iter()
        .map(|field| (field.name.as_str(), field.value.as_str()))
        .collect()
}

fn compact_failure_output(output: Output) -> Vec<CompactRecord> {
    compact_failure_output_with_status(output, 2)
}

fn compact_failure_output_with_status(output: Output, expected_status: i32) -> Vec<CompactRecord> {
    assert_eq!(output.status.code(), Some(expected_status));
    assert!(output.stderr.is_empty());
    assert!(output.stdout.len() < 16 * 1024);
    let records = parse_records("stdout", &output.stdout).expect("compact failure records");
    assert_eq!(compact_field(&records[0], "status"), Some("failure"));
    records
}

fn current_revision(project: &Path) -> String {
    current_revision_at(&binary(), Path::new(env!("CARGO_MANIFEST_DIR")), project)
}

fn current_revision_at(executable: &Path, directory: &Path, project: &Path) -> String {
    let status = compact_success_at(
        executable,
        directory,
        &["--project", path(project), "status"],
    );
    compact_field(compact_record(&status, "revision"), "id")
        .expect("current revision")
        .to_owned()
}

fn assert_direct_rename_rejection(
    project: &Path,
    arguments: &[&str],
    expected_code: &str,
    expected_status: i32,
    unchanged_revision: &str,
) {
    let rejected = compact_failure_output_with_status(command(arguments), expected_status);
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some(expected_code)
    );
    assert_eq!(current_revision(project), unchanged_revision);
}

fn content_inventory(root: &Path) -> BTreeMap<String, [u8; 32]> {
    fn visit(root: &Path, directory: &Path, output: &mut BTreeMap<String, [u8; 32]>) {
        let mut entries = std::fs::read_dir(directory)
            .expect("read inventory directory")
            .collect::<Result<Vec<_>, _>>()
            .expect("read inventory entries");
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let metadata = entry.metadata().expect("inventory metadata");
            if metadata.is_dir() {
                visit(root, &path, output);
            } else if metadata.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .expect("inventory path under root")
                    .to_string_lossy()
                    .into_owned();
                let bytes = std::fs::read(&path).expect("inventory file");
                output.insert(relative, *blake3::hash(&bytes).as_bytes());
            }
        }
    }
    let mut output = BTreeMap::new();
    visit(root, root, &mut output);
    output
}

#[test]
fn product_version_is_exact_and_has_no_alias_or_mixed_form() {
    let version = command(&["--version"]);
    assert_eq!(version.status.code(), Some(0));
    assert_eq!(
        version.stdout,
        format!("lkjscript {}\n", lkjscript::PRODUCT_VERSION).into_bytes()
    );
    assert!(version.stderr.is_empty());

    for arguments in [
        vec!["version"],
        vec!["-V"],
        vec!["--version=0.1.10"],
        vec!["--version", "extra"],
        vec!["--project", APPLICATION, "--version"],
    ] {
        let rejected = compact_failure_output(command(&arguments));
        assert_eq!(
            compact_field(compact_record(&rejected, "diagnostic"), "code"),
            Some("cli_usage"),
            "{arguments:?}"
        );
    }
}

#[test]
fn capabilities_discovery_is_compact_focused_and_exportable() {
    let capabilities = compact_success(&["capabilities"]);
    assert_eq!(capabilities[0].operation, "product");
    assert_eq!(
        compact_record_values(&capabilities[0]),
        vec![
            ("name", "lkjscript"),
            ("version", lkjscript::PRODUCT_VERSION)
        ]
    );
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
            "package",
            "check",
            "build",
            "run",
            "serve",
            "worker"
        ]
    );
    assert!(
        !capabilities
            .iter()
            .any(|record| record.operation == "registry")
    );
    let capability_record = compact_record(&capabilities, "capabilities");
    let capabilities_digest =
        compact_field(capability_record, "digest").expect("capabilities digest");
    assert_eq!(capabilities_digest.len(), 64);
    let cached = compact_success(&["capabilities", "--known-capabilities", capabilities_digest]);
    assert_eq!(
        compact_field(compact_record(&cached, "capabilities"), "unchanged"),
        Some("true")
    );
    assert_eq!(cached.len(), 3);
    let stale_digest = if capabilities_digest == "0".repeat(64) {
        "1".repeat(64)
    } else {
        "0".repeat(64)
    };
    let stale = compact_success(&["capabilities", "--known-capabilities", &stale_digest]);
    assert_eq!(
        compact_field(compact_record(&stale, "capabilities"), "unchanged"),
        Some("false")
    );
    assert!(stale.len() > cached.len());
    let malformed = compact_failure_output(command(&[
        "capabilities",
        "--known-capabilities",
        "not-a-digest",
    ]));
    assert_eq!(
        compact_field(compact_record(&malformed, "diagnostic"), "code"),
        Some("cli_usage")
    );
    let implicit = compact_success_output(command(&[]));
    assert_eq!(
        compact_field(compact_record(&implicit, "capabilities"), "digest"),
        Some(capabilities_digest)
    );
    let project_scoped = compact_success(&[
        "--project",
        APPLICATION,
        "capabilities",
        "--known-capabilities",
        capabilities_digest,
    ]);
    assert_eq!(
        compact_field(compact_record(&project_scoped, "capabilities"), "unchanged"),
        Some("true")
    );

    for predecessor in [
        vec!["capabilities", "--known-registry", capabilities_digest],
        vec!["capabilities", "--section", "contracts"],
    ] {
        let rejected = compact_failure_output(command(&predecessor));
        assert_eq!(
            compact_field(compact_record(&rejected, "diagnostic"), "code"),
            Some("cli_usage"),
            "{predecessor:?}"
        );
    }

    let type_section = compact_success(&["capabilities", "--section", "type"]);
    assert!(type_section.iter().any(|record| {
        record.operation == "type.form" && compact_field(record, "name") == Some("parameter")
    }));
    assert!(type_section.iter().any(|record| {
        record.operation == "type.form"
            && compact_field(record, "name") == Some("structural-record")
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
    for expected in [
        "call",
        "constant",
        "function-value",
        "invoke",
        "let",
        "record",
        "variant",
        "field",
        "list",
        "match",
        "capability-call",
        "transaction",
    ] {
        assert!(expression_section.iter().any(|record| {
            record.operation == "expression.form" && compact_field(record, "name") == Some(expected)
        }));
    }

    let change_section = compact_success(&["capabilities", "--section", "change"]);
    let change_contract = compact_record(&change_section, "change");
    assert_eq!(
        compact_field(change_contract, "plan-hex-characters"),
        Some("128")
    );
    assert_eq!(
        compact_field(change_contract, "request-commitment"),
        Some("opaque-digest")
    );
    assert_eq!(
        compact_field(change_contract, "prepared-plan"),
        Some("opaque-commitment")
    );
    assert_eq!(
        compact_field(change_contract, "plan-output-action"),
        Some("plan-only")
    );
    assert_eq!(
        change_section
            .iter()
            .filter(|record| record.operation == "change.plan-record")
            .count(),
        29
    );
    assert!(change_section.iter().any(|record| {
        record.operation == "change.plan-record-field"
            && compact_field(record, "record") == Some("logical-plan.digest")
            && compact_field(record, "name") == Some("token")
    }));
    let operations = change_section
        .iter()
        .filter(|record| record.operation == "change.operation")
        .filter_map(|record| compact_field(record, "name"))
        .collect::<Vec<_>>();
    assert_eq!(
        operations,
        vec![
            "create.module",
            "create.record",
            "create.variant",
            "create.function",
            "create.constant",
            "create.test",
            "add.field",
            "add.case",
            "add.type-parameter",
            "add.parameter",
            "add.requirement",
            "set.function-contract",
            "delete.owner",
            "rename.owner",
            "move.declaration",
            "replace.body",
        ]
    );
    let operation_fields = change_section
        .iter()
        .filter(|record| record.operation == "change.operation-field")
        .collect::<Vec<_>>();
    assert_eq!(operation_fields.len(), 60);
    assert_eq!(
        operation_fields
            .iter()
            .filter(|record| compact_field(record, "required") == Some("false"))
            .map(|record| {
                (
                    compact_field(record, "operation").expect("field operation"),
                    compact_field(record, "name").expect("field name"),
                )
            })
            .collect::<Vec<_>>(),
        vec![("add.case", "payload")]
    );
    let field_forms = change_section
        .iter()
        .filter(|record| record.operation == "change.field-form")
        .filter_map(|record| compact_field(record, "name"))
        .collect::<Vec<_>>();
    assert_eq!(field_forms.len(), 18);
    let name_form = change_section
        .iter()
        .find(|record| {
            record.operation == "change.field-form" && compact_field(record, "name") == Some("name")
        })
        .expect("name field form");
    assert_eq!(
        compact_field(name_form, "syntax"),
        Some("[A-Za-z_][A-Za-z0-9_-]{0,127}")
    );
    let type_reference = change_section
        .iter()
        .find(|record| {
            record.operation == "change.field-form"
                && compact_field(record, "name") == Some("type_reference")
        })
        .expect("type reference field form");
    assert_eq!(
        compact_field(type_reference, "syntax"),
        Some("unit|bool|i64|bytes|text|static-text|secret|@NAME")
    );
    let query = compact_success(&["capabilities", "query"]);
    assert_eq!(
        query
            .iter()
            .filter(|record| record.operation == "query.operation")
            .filter_map(|record| compact_field(record, "name"))
            .collect::<Vec<_>>(),
        vec!["owners", "find", "relations"]
    );
    assert!(query.iter().any(|record| {
        record.operation == "query.owner-kind"
            && compact_field(record, "name") == Some("expression")
    }));
    assert!(query.iter().any(|record| {
        record.operation == "query.namespace-class"
            && compact_field(record, "name") == Some("parameter")
    }));
    assert!(query.iter().any(|record| {
        record.operation == "query.relation-kind"
            && compact_field(record, "name") == Some("function_call")
    }));
    assert_eq!(
        query
            .iter()
            .filter(|record| record.operation == "query.owner-kind")
            .count(),
        22
    );
    assert_eq!(
        query
            .iter()
            .filter(|record| record.operation == "query.namespace-class")
            .count(),
        10
    );
    assert_eq!(
        query
            .iter()
            .filter(|record| record.operation == "query.relation-kind")
            .count(),
        27
    );
    assert_eq!(
        query
            .iter()
            .filter(|record| record.operation == "query.direction")
            .count(),
        2
    );
    assert_eq!(
        query
            .iter()
            .filter(|record| record.operation == "query.response-field")
            .count(),
        35
    );
    assert_eq!(
        query
            .iter()
            .filter(|record| record.operation == "query.selector-field")
            .count(),
        9
    );
    for (name, value) in [
        ("default-items", "50"),
        ("maximum-items", "10000"),
        ("minimum-output-bytes", "1536"),
        ("default-output-bytes", "65536"),
        ("maximum-output-bytes", "4194304"),
        ("maximum-continuation-bytes", "320"),
    ] {
        assert!(query.iter().any(|record| {
            record.operation == "query.limit"
                && compact_field(record, "name") == Some(name)
                && compact_field(record, "value") == Some(value)
        }));
    }
    assert!(!query.iter().any(|record| {
        matches!(
            compact_field(record, "name"),
            Some("callers" | "callees" | "context" | "impact" | "request")
        )
    }));
    for field in change_section.iter().filter(|record| {
        matches!(
            record.operation.as_str(),
            "change.operation-field" | "change.precondition-field"
        )
    }) {
        assert!(
            field_forms.contains(&compact_field(field, "form").expect("field form")),
            "unadvertised change field form in {field:?}"
        );
    }
    let references = change_section
        .iter()
        .filter(|record| record.operation == "change.reference")
        .collect::<Vec<_>>();
    assert_eq!(references.len(), 8);
    let exact_reference = references
        .iter()
        .find(|record| compact_field(record, "name") == Some("exact_package_declaration"))
        .expect("exact reference form");
    assert_eq!(
        compact_field(exact_reference, "syntax"),
        Some("pkg_HEX/decl_HEX")
    );
    assert_eq!(
        change_section
            .iter()
            .filter(|record| record.operation == "change.delete-policy")
            .filter_map(|record| compact_field(record, "name"))
            .collect::<Vec<_>>(),
        vec!["reject", "owned-closure"]
    );
    assert!(change_section.iter().any(|record| {
        record.operation == "change.operation-field"
            && compact_field(record, "operation") == Some("delete.owner")
            && compact_field(record, "name") == Some("owner")
            && compact_field(record, "required") == Some("true")
            && compact_field(record, "form") == Some("exact_owner")
    }));
    assert_eq!(
        change_section
            .iter()
            .filter(|record| record.operation == "change.declaration-visibility")
            .filter_map(|record| compact_field(record, "name"))
            .collect::<Vec<_>>(),
        vec!["private", "package", "public"]
    );
    assert_eq!(
        change_section
            .iter()
            .filter(|record| record.operation == "change.function-effect")
            .filter_map(|record| compact_field(record, "name"))
            .collect::<Vec<_>>(),
        vec!["pure", "task"]
    );

    let deployment_section = compact_success(&["capabilities", "--section", "deployment"]);
    assert!(deployment_section.iter().any(|record| {
        record.operation == "deployment.adapter"
            && compact_field(record, "kind") == Some("postgres")
    }));
    for field in [
        "connection_secret",
        "maximum_connections",
        "maximum_wait_milliseconds",
        "statement_timeout_milliseconds",
    ] {
        assert!(deployment_section.iter().any(|record| {
            record.operation == "deployment.adapter-field"
                && compact_field(record, "adapter") == Some("postgres")
                && compact_field(record, "path")
                    == Some(format!("adapter.postgres.{field}").as_str())
        }));
    }
    let direct = change_section
        .iter()
        .filter(|record| record.operation == "change.direct-operation")
        .collect::<Vec<_>>();
    assert_eq!(direct.len(), 1);
    assert_eq!(compact_field(direct[0], "name"), Some("rename.owner"));
    assert_eq!(
        compact_field(direct[0], "plan-usage"),
        Some(
            "change plan rename.owner --base REVISION --owner OWNER --name NAME [--idempotency KEY] [--intent TEXT] [--output PATH]"
        )
    );
    assert_eq!(
        compact_field(direct[0], "apply-usage"),
        Some(
            "change apply rename.owner --base REVISION --owner OWNER --name NAME [--idempotency KEY] [--intent TEXT] --plan PLAN"
        )
    );
    assert!(change_section.iter().any(|record| {
        record.operation == "change.operation-field"
            && compact_field(record, "operation") == Some("delete.owner")
            && compact_field(record, "name") == Some("policy")
            && compact_field(record, "required") == Some("true")
            && compact_field(record, "form") == Some("delete_policy")
    }));
    assert!(change_section.iter().any(|record| {
        record.operation == "change.precondition"
            && compact_field(record, "name") == Some("precondition.owner-exists")
    }));
    assert!(change_section.iter().any(|record| {
        record.operation == "change.precondition-field"
            && compact_field(record, "precondition") == Some("precondition.owner-exists")
            && compact_field(record, "name") == Some("owner")
            && compact_field(record, "required") == Some("true")
            && compact_field(record, "form") == Some("exact_owner")
    }));
    assert!(change_section.iter().any(|record| {
        record.operation == "change.precondition-field"
            && compact_field(record, "precondition") == Some("precondition.dependency-binding")
            && compact_field(record, "name") == Some("package-revision")
            && compact_field(record, "form") == Some("exact_package_revision")
    }));
    assert!(change_section.iter().any(|record| {
        record.operation == "change.namespace-class"
            && compact_field(record, "name") == Some("declaration")
    }));
    assert!(change_section.iter().any(|record| {
        record.operation == "change.parent-form"
            && compact_field(record, "name") == Some("package")
            && compact_field(record, "syntax") == Some("package")
    }));
    let known_type_digest = compact_field(compact_record(&type_section, "section"), "digest")
        .expect("type section digest");
    let known_type = format!("type={known_type_digest}");
    let unchanged_type = compact_success(&["capabilities", "--known-section", &known_type]);
    assert_eq!(
        compact_field(compact_record(&unchanged_type, "capabilities"), "unchanged"),
        Some("true")
    );
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
    assert!(change_usage.contains("--output PATH"));
    assert!(change_usage.contains("--plan TOKEN"));

    let new_help = compact_success(&["capabilities", "new"]);
    let new_operation = compact_record(&new_help, "operation");
    assert_eq!(
        compact_field(new_operation, "response-model"),
        Some("new_result")
    );
    assert_eq!(
        compact_field(new_operation, "usage"),
        Some("new DEST [--template minimal|command|http] [--name NAME]")
    );
    assert_eq!(
        new_help
            .iter()
            .filter(|record| record.operation == "template")
            .filter_map(|record| compact_field(record, "name"))
            .collect::<Vec<_>>(),
        vec!["minimal", "command", "http"]
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
        vec!["minimal", "command", "http"]
    );
    let http_template = templates
        .iter()
        .find(|record| {
            record.operation == "template" && compact_field(record, "name") == Some("http")
        })
        .expect("HTTP template descriptor");
    assert_eq!(compact_field(http_template, "runner"), Some("http"));
    assert_eq!(
        compact_field(http_template, "starter-deployment"),
        Some("true")
    );
    assert_eq!(
        compact_field(http_template, "recommended-artifact-output"),
        Some("generated/application.lkja")
    );
    for name in ["minimal", "command"] {
        let template = templates
            .iter()
            .find(|record| {
                record.operation == "template" && compact_field(record, "name") == Some(name)
            })
            .expect("non-HTTP template descriptor");
        assert_eq!(compact_field(template, "starter-deployment"), Some("false"));
        assert_eq!(
            compact_field(template, "recommended-artifact-output"),
            Some("none")
        );
    }

    let temporary_capabilities = tempfile::TempDir::new().expect("capabilities output");
    let capabilities_path = temporary_capabilities.path().join("capabilities.lkjc");
    let capabilities_path_text = path(&capabilities_path);
    let exported = compact_success(&["capabilities", "--output", capabilities_path_text]);
    let file = compact_record(&exported, "file");
    assert_eq!(compact_field(file, "kind"), Some("capabilities"));
    assert_eq!(compact_field(file, "digest"), Some(capabilities_digest));
    let exported_bytes = std::fs::read(&capabilities_path).expect("compact capabilities export");
    let exported_records =
        parse_records("capabilities.lkjc", &exported_bytes).expect("export records");
    assert!(exported_records.len() > capabilities.len());
    assert_ne!(exported_bytes.first(), Some(&b'{'));
    assert_eq!(exported_records[0].operation, "product");
    assert!(!exported_records.iter().any(|record| {
        record.operation == "contract"
            || record.operation == "contract.magic"
            || record.operation == "contract.digest"
            || record.operation == "logical-plan.contracts"
            || record.fields.iter().any(|field| {
                matches!(
                    field.name.as_str(),
                    "contract" | "contract-version" | "cli" | "graph"
                )
            })
    }));
    let exported_text = String::from_utf8(exported_bytes).expect("capabilities UTF-8");
    for forbidden in [
        "lkjscript-contract-registry-",
        "lkjscript-meaning-graph-",
        "lkjscript-cli-",
        "lkjscript-change-records-",
        "lkjscript-query-",
        "lkjscript-deployment-",
        "logical-plan.contracts",
        "contract_version",
        "contract=",
    ] {
        assert!(!exported_text.contains(forbidden), "leaked {forbidden}");
    }

    let rejected_schema = command(&["capabilities", "--known-schema", capabilities_digest]);
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
fn generated_public_guides_match_executable() {
    let verified = compact_success(&["capabilities", "--verify-generated", "docs/generated"]);
    assert_eq!(
        verified
            .iter()
            .filter(|record| record.operation == "file")
            .count(),
        6
    );
    assert!(!Path::new("docs/generated/contracts.md").exists());
}

#[test]
fn builtin_owner_discovery_is_exact_bounded_and_revision_bound() {
    let database = compact_success(&[
        "package",
        "builtin",
        "query",
        "owners",
        "--kind",
        "interface",
        "--name",
        "Database",
        "--limit",
        "1",
        "--bytes",
        "65536",
    ]);
    let package_revision = compact_field(compact_record(&database, "package"), "package-revision")
        .expect("built-in package revision");
    assert_eq!(
        compact_field(compact_record(&database, "query"), "package-revision"),
        Some(package_revision)
    );
    let owner = compact_record(&database, "owner");
    assert_eq!(compact_field(owner, "kind"), Some("interface"));
    assert_eq!(compact_field(owner, "name"), Some("Database"));
    let identity = compact_field(owner, "id").expect("database interface identity");
    let reference = compact_field(owner, "reference").expect("database interface reference");
    assert!(reference.ends_with(identity));

    let detail = compact_success(&[
        "package",
        "builtin",
        "inspect",
        "owner",
        "interface",
        identity,
    ]);
    for (name, idempotency, visibility) in [
        ("execute", "idempotent-with-key", "possible"),
        ("migration", "idempotent-with-key", "possible"),
        ("transaction", "idempotent-with-key", "possible"),
        ("query", "idempotent", "none"),
    ] {
        assert!(detail.iter().any(|record| {
            record.operation == "operation"
                && compact_field(record, "name") == Some(name)
                && compact_field(record, "idempotency") == Some(idempotency)
                && compact_field(record, "external-visibility") == Some(visibility)
        }));
    }
    assert!(detail.iter().any(|record| {
        record.operation == "type"
            && compact_field(record, "path") == Some("parameter.statement")
            && compact_field(record, "form") == Some("static-text")
    }));

    let first = compact_success(&[
        "package",
        "builtin",
        "query",
        "owners",
        "--kind",
        "interface",
        "--limit",
        "1",
        "--bytes",
        "1536",
    ]);
    assert_eq!(
        compact_field(compact_record(&first, "summary"), "truncated"),
        Some("true")
    );
    let first_owner = compact_field(compact_record(&first, "owner"), "id")
        .expect("first paged owner")
        .to_owned();
    let token = compact_field(compact_record(&first, "continuation"), "token")
        .expect("built-in continuation")
        .to_owned();
    let resumed = compact_success(&[
        "package",
        "builtin",
        "query",
        "owners",
        "--kind",
        "interface",
        "--limit",
        "1",
        "--bytes",
        "1536",
        "--continuation",
        &token,
    ]);
    assert_ne!(
        compact_field(compact_record(&resumed, "owner"), "id"),
        Some(first_owner.as_str())
    );

    let mismatch = compact_failure_output(command(&[
        "package",
        "builtin",
        "query",
        "owners",
        "--kind",
        "interface",
        "--name",
        "Database",
        "--limit",
        "1",
        "--bytes",
        "1536",
        "--continuation",
        &token,
    ]));
    assert_eq!(
        compact_field(compact_record(&mismatch, "diagnostic"), "code"),
        Some("builtin_continuation_selector")
    );
}

#[test]
fn normalized_query_and_maintained_check_build_are_dependency_closed() {
    let normalized = tempfile::TempDir::new().expect("normalized query project");
    let project = normalized.path().join("project");
    compact_success(&["new", path(&project), "--name", "query-check"]);
    let owners = compact_success(&["--project", path(&project), "query", "owners"]);
    assert_eq!(
        compact_field(compact_record(&owners, "summary"), "returned"),
        Some("0")
    );
    assert_eq!(
        compact_field(compact_record(&owners, "summary"), "truncated"),
        Some("false")
    );

    let tests = compact_success(&["--project", APPLICATION, "check"]);
    assert_eq!(
        compact_field(compact_record(&tests, "tests"), "passed"),
        Some("16")
    );
    assert_eq!(
        compact_field(compact_record(&tests, "tests"), "differential"),
        Some("equal")
    );

    let temporary = tempfile::TempDir::new().expect("temporary output");
    let artifact = temporary.path().join("application.lkja");
    let artifact_text = path(&artifact);
    let first = compact_success(&["--project", APPLICATION, "build", "--output", artifact_text]);
    assert_eq!(
        compact_field(compact_record(&first, "output"), "visibility"),
        Some("created")
    );
    assert_eq!(
        compact_field(compact_record(&first, "artifact"), "packages"),
        Some("2")
    );
    let repeated = compact_failure_output(command(&[
        "--project",
        APPLICATION,
        "build",
        "--output",
        artifact_text,
    ]));
    assert_eq!(
        compact_field(compact_record(&repeated, "diagnostic"), "code"),
        Some("output_conflict")
    );
}

#[test]
fn copied_binary_rediscovers_normalized_names_and_relations_without_query_writes() {
    let temporary = tempfile::TempDir::new().expect("isolated normalized query workspace");
    let copied_binary = temporary.path().join("lkjscript");
    copy_executable(&binary(), &copied_binary);
    let project = temporary.path().join("project");
    let created = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&project), "--name", "query-workflow"],
    );
    let initial_revision = compact_field(compact_record(&created, "revision"), "id")
        .expect("initial normalized revision")
        .to_owned();
    let change = format!(
        "request base={initial_revision} idempotency=query-workflow-create\n\
         create.module as=$alpha name=alpha\n\
         create.module as=$beta name=beta\n\
         create.record as=$payload module=$alpha name=Payload visibility=public\n\
         add.field as=$value record=$payload name=value type=unit\n\
         expression.unit as=$callee-body\n\
         create.function as=$callee module=$alpha name=callee visibility=public result=unit effect=pure body=$callee-body\n\
         expression.call as=$call function=$callee\n\
         create.function as=$caller module=$beta name=caller visibility=public result=unit effect=pure body=$call\n\
         expression.call as=$call-two function=$callee\n\
         create.function as=$other-caller module=$beta name=other_caller visibility=public result=unit effect=pure body=$call-two\n"
    );
    let change_path = temporary.path().join("create.lkjc");
    std::fs::write(&change_path, change).expect("normalized query topology request");
    let planned = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "--input-file",
            path(&change_path),
        ],
    );
    let plan = compact_field(compact_record(&planned, "plan"), "token")
        .expect("topology plan")
        .to_owned();
    let applied = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "--input-file",
            path(&change_path),
            "--plan",
            &plan,
        ],
    );
    let accepted_revision = compact_field(compact_record(&applied, "revision"), "result")
        .expect("accepted topology revision")
        .to_owned();
    drop(planned);
    drop(applied);

    let before_queries = content_inventory(&project);
    let nested = project.join("ordinary/nested");
    std::fs::create_dir_all(&nested).expect("nested normalized query directory");
    let alpha = compact_success_at(
        &copied_binary,
        &nested,
        &["query", "find", "module", "alpha"],
    );
    assert_eq!(
        compact_field(compact_record(&alpha, "summary"), "match"),
        Some("true")
    );
    let alpha_id = compact_field(compact_record(&alpha, "owner"), "id")
        .expect("rediscovered alpha module")
        .to_owned();
    let beta = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "module",
            "beta",
        ],
    );
    let beta_id = compact_field(compact_record(&beta, "owner"), "id")
        .expect("rediscovered beta module")
        .to_owned();
    let callee = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "declaration",
            "callee",
            "--parent",
            &alpha_id,
        ],
    );
    let callee_id = compact_field(compact_record(&callee, "owner"), "id")
        .expect("rediscovered callee")
        .to_owned();
    let caller = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "declaration",
            "caller",
            "--parent",
            &beta_id,
        ],
    );
    let caller_id = compact_field(compact_record(&caller, "owner"), "id")
        .expect("rediscovered caller")
        .to_owned();
    let payload = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "declaration",
            "Payload",
            "--parent",
            &alpha_id,
        ],
    );
    let payload_id = compact_field(compact_record(&payload, "owner"), "id")
        .expect("rediscovered record")
        .to_owned();
    let field = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "field",
            "value",
            "--parent",
            &payload_id,
        ],
    );
    let field_id = compact_field(compact_record(&field, "owner"), "id")
        .expect("rediscovered nested field")
        .to_owned();

    let first_page = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "owners",
            "--limit",
            "2",
        ],
    );
    let stale_candidate = compact_field(compact_record(&first_page, "continuation"), "token")
        .expect("first owner continuation")
        .to_owned();
    let mut owner_ids = first_page
        .iter()
        .filter(|record| record.operation == "owner")
        .filter_map(|record| compact_field(record, "id"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut continuation = Some(stale_candidate.clone());
    while let Some(token) = continuation {
        let page = compact_success_at(
            &copied_binary,
            temporary.path(),
            &[
                "--project",
                path(&project),
                "query",
                "owners",
                "--limit",
                "3",
                "--continuation",
                &token,
            ],
        );
        owner_ids.extend(
            page.iter()
                .filter(|record| record.operation == "owner")
                .filter_map(|record| compact_field(record, "id"))
                .map(str::to_owned),
        );
        continuation = page
            .iter()
            .find(|record| record.operation == "continuation")
            .and_then(|record| compact_field(record, "token"))
            .map(str::to_owned);
    }
    for recovered in [
        &alpha_id,
        &beta_id,
        &callee_id,
        &caller_id,
        &payload_id,
        &field_id,
    ] {
        assert!(owner_ids.contains(recovered));
    }
    let first_relations = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "relations",
            &callee_id,
            "--direction",
            "incoming",
            "--kind",
            "function_call",
            "--limit",
            "1",
        ],
    );
    let first_relation_token =
        compact_field(compact_record(&first_relations, "continuation"), "token")
            .expect("paginated incoming relation continuation")
            .to_owned();
    let remaining_relations = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "relations",
            &callee_id,
            "--direction",
            "incoming",
            "--kind",
            "function_call",
            "--limit",
            "2",
            "--continuation",
            &first_relation_token,
        ],
    );
    let calls = first_relations
        .iter()
        .chain(&remaining_relations)
        .filter(|record| record.operation == "relation")
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 2);
    assert!(
        remaining_relations
            .iter()
            .all(|record| record.operation != "continuation")
    );
    for call in calls {
        assert_eq!(compact_field(call, "kind"), Some("function_call"));
        assert_eq!(
            compact_field(call, "target-owner"),
            Some(callee_id.as_str())
        );
        assert!(compact_field(call, "source-owner").is_some());
    }
    let outgoing = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "relations",
            &caller_id,
            "--direction",
            "outgoing",
            "--kind",
            "declaration_module",
        ],
    );
    assert_eq!(
        compact_field(compact_record(&outgoing, "relation"), "target-owner"),
        Some(beta_id.as_str())
    );
    for direction in ["incoming", "outgoing"] {
        let package_relations = compact_success_at(
            &copied_binary,
            temporary.path(),
            &[
                "--project",
                path(&project),
                "query",
                "relations",
                "package",
                "--direction",
                direction,
                "--kind",
                "package_dependency",
            ],
        );
        assert_eq!(
            compact_field(compact_record(&package_relations, "summary"), "returned"),
            Some("0")
        );
    }
    let mismatched = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "owners",
            "--kind",
            "module",
            "--continuation",
            &stale_candidate,
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&mismatched, "diagnostic"), "code"),
        Some("query_continuation_mismatch")
    );
    let malformed = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "owners",
            "--continuation",
            "qcont_bad!",
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&malformed, "diagnostic"), "code"),
        Some("query_continuation_malformed")
    );
    let predecessor_token = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "owners",
            "--continuation",
            "cont_predecessor",
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&predecessor_token, "diagnostic"), "code"),
        Some("predecessor_contract")
    );
    for (arguments, expected_code) in [
        (
            vec![
                "--project",
                path(&project),
                "query",
                "owners",
                "--bytes",
                "1",
            ],
            "query_invalid_byte_limit",
        ),
        (
            vec![
                "--project",
                path(&project),
                "query",
                "owners",
                "--continue",
                &stale_candidate,
            ],
            "query_unknown_option",
        ),
        (
            vec![
                "--project",
                path(&project),
                "query",
                "owners",
                "--work",
                "1",
            ],
            "query_unknown_option",
        ),
    ] {
        let rejected =
            compact_failure_output(command_at(&copied_binary, temporary.path(), &arguments));
        assert_eq!(
            compact_field(compact_record(&rejected, "diagnostic"), "code"),
            Some(expected_code)
        );
    }
    let absent_owner = if alpha_id == "mod_00000000000000000000000000000001" {
        "mod_00000000000000000000000000000002"
    } else {
        "mod_00000000000000000000000000000001"
    };
    let absent_relation = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "relations",
            absent_owner,
            "--direction",
            "outgoing",
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&absent_relation, "diagnostic"), "code"),
        Some("query_owner_not_found")
    );
    let foreign_project = temporary.path().join("foreign");
    compact_success_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&foreign_project), "--name", "foreign"],
    );
    let foreign = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&foreign_project),
            "query",
            "owners",
            "--continuation",
            &stale_candidate,
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&foreign, "diagnostic"), "code"),
        Some("query_continuation_foreign")
    );
    assert_eq!(content_inventory(&project), before_queries);
    assert_eq!(
        current_revision_at(&copied_binary, temporary.path(), &project),
        accepted_revision
    );

    let rename_plan = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "rename.owner",
            "--base",
            &accepted_revision,
            "--owner",
            &callee_id,
            "--name",
            "renamed",
        ],
    );
    let rename_digest = compact_field(compact_record(&rename_plan, "plan"), "token")
        .expect("rename plan")
        .to_owned();
    let renamed = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "rename.owner",
            "--base",
            &accepted_revision,
            "--owner",
            &callee_id,
            "--name",
            "renamed",
            "--plan",
            &rename_digest,
        ],
    );
    let renamed_revision = compact_field(compact_record(&renamed, "revision"), "result")
        .expect("renamed revision")
        .to_owned();
    let after_rename = content_inventory(&project);
    let old_name = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "declaration",
            "callee",
            "--parent",
            &alpha_id,
        ],
    );
    assert_eq!(
        compact_field(compact_record(&old_name, "summary"), "match"),
        Some("false")
    );
    let new_name = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "declaration",
            "renamed",
            "--parent",
            &alpha_id,
        ],
    );
    assert_eq!(
        compact_field(compact_record(&new_name, "owner"), "id"),
        Some(callee_id.as_str())
    );
    let stale = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "owners",
            "--continuation",
            &stale_candidate,
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&stale, "diagnostic"), "code"),
        Some("query_continuation_stale")
    );
    assert_eq!(content_inventory(&project), after_rename);
    assert_eq!(
        current_revision_at(&copied_binary, temporary.path(), &project),
        renamed_revision
    );

    for action in [
        "callers",
        "callees",
        "types",
        "capabilities",
        "context",
        "impact",
        "request",
    ] {
        let rejected = compact_failure_output(command_at(
            &copied_binary,
            temporary.path(),
            &["--project", path(&project), "query", action],
        ));
        assert_eq!(
            compact_field(compact_record(&rejected, "diagnostic"), "code"),
            Some("predecessor_contract")
        );
    }
    for request_option in ["--request", "--request-file", "--file"] {
        let rejected = compact_failure_output(command_at(
            &copied_binary,
            temporary.path(),
            &[
                "--project",
                path(&project),
                "query",
                "request",
                request_option,
                "{}",
            ],
        ));
        assert_eq!(
            compact_field(compact_record(&rejected, "diagnostic"), "code"),
            Some("predecessor_contract")
        );
    }
    let predecessor = temporary.path().join("predecessor");
    std::fs::create_dir_all(predecessor.join(".lkjscript/meaning"))
        .expect("predecessor marker directory");
    std::fs::write(
        predecessor.join(".lkjscript/meaning/HEAD"),
        b"predecessor\n",
    )
    .expect("predecessor marker");
    let rejected = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&predecessor), "query", "owners"],
    ));
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some("predecessor_contract")
    );
}

#[test]
fn interrupted_copied_query_after_reads_and_rendering_writes_no_repository_bytes() {
    let temporary = tempfile::TempDir::new().expect("isolated interrupted query workspace");
    let copied_binary = temporary.path().join("lkjscript");
    copy_executable(&binary(), &copied_binary);
    let project = temporary.path().join("project");
    let created = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&project), "--name", "interrupted-query"],
    );
    let initial = compact_field(compact_record(&created, "revision"), "id")
        .expect("interruption fixture revision")
        .to_owned();
    let mut request = format!("request base={initial}\n");
    for ordinal in 0..1_000_u64 {
        request.push_str(&format!(
            "create.module as=$module-{ordinal:04} name=module_{ordinal:04}\n"
        ));
    }
    let request_path = temporary.path().join("large-query-fixture.lkjc");
    std::fs::write(&request_path, request).expect("large normalized query fixture request");

    let planned = command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "--input-file",
            path(&request_path),
        ],
    );
    assert!(planned.status.success(), "large query fixture plan failed");
    assert!(planned.stderr.is_empty());
    let planned_records =
        parse_records("large fixture plan", &planned.stdout).expect("large fixture plan records");
    let plan = compact_field(compact_record(&planned_records, "plan"), "token")
        .expect("large fixture plan digest")
        .to_owned();
    let applied = command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "--input-file",
            path(&request_path),
            "--plan",
            &plan,
        ],
    );
    assert!(applied.status.success(), "large query fixture apply failed");
    assert!(applied.stderr.is_empty());
    let applied_records =
        parse_records("large fixture apply", &applied.stdout).expect("large fixture apply records");
    let accepted = compact_field(compact_record(&applied_records, "revision"), "result")
        .expect("large fixture accepted revision")
        .to_owned();
    drop(planned_records);
    drop(applied_records);
    drop(planned);
    drop(applied);

    let before = content_inventory(&project);
    let mut child = Command::new(&copied_binary)
        .args([
            "--project",
            path(&project),
            "query",
            "owners",
            "--limit",
            "1000",
            "--bytes",
            "4194304",
        ])
        .current_dir(temporary.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn interruptible copied query");
    let mut stdout = child.stdout.take().expect("interruptible query stdout");
    let mut first_output_byte = [0_u8; 1];
    stdout
        .read_exact(&mut first_output_byte)
        .expect("query reached completed rendering and process output");
    child
        .kill()
        .expect("large query must still be blocked on its unread compact output");
    drop(stdout);
    let interrupted = child
        .wait_with_output()
        .expect("wait for interrupted copied query");
    assert!(!interrupted.status.success());
    assert!(interrupted.stderr.is_empty());
    assert_eq!(content_inventory(&project), before);
    assert_eq!(
        current_revision_at(&copied_binary, temporary.path(), &project),
        accepted
    );
}

#[test]
fn copied_binary_completes_normalized_standard_dependent_command_lifecycle() {
    let temporary = tempfile::TempDir::new().expect("isolated binary workspace");
    let copied_binary = temporary.path().join("lkjscript");
    copy_executable(&binary(), &copied_binary);
    let project = temporary.path().join("app");

    let created = compact_success_at(
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
    let initial_revision = compact_field(compact_record(&created, "revision"), "id")
        .expect("created revision")
        .to_owned();
    let creation_summary = compact_record(&created, "summary");
    assert_eq!(compact_field(creation_summary, "dependencies"), Some("1"));
    assert_eq!(compact_field(creation_summary, "targets"), Some("1"));
    assert_eq!(compact_field(creation_summary, "tests"), Some("1"));
    assert!(!project.join(".lkjscript").exists());
    assert!(!temporary.path().join("Cargo.toml").exists());

    let status = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "status"],
    );
    assert_eq!(
        compact_field(compact_record(&status, "revision"), "id"),
        Some(initial_revision.as_str())
    );
    assert_eq!(
        compact_field(compact_record(&status, "summary"), "dependencies"),
        Some("1")
    );
    let application = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "module",
            "application",
        ],
    );
    let application_owner = compact_field(compact_record(&application, "owner"), "id")
        .expect("application module")
        .to_owned();
    let inspected = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "inspect",
            "owner",
            "module",
            &application_owner,
        ],
    );
    assert_eq!(
        compact_field(compact_record(&inspected, "owner"), "name"),
        Some("application")
    );

    let head_before_check = std::fs::read(project.join("HEAD")).expect("initial HEAD");
    let checked = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "check"],
    );
    let tests = compact_record(&checked, "tests");
    assert_eq!(compact_field(tests, "passed"), Some("12"));
    assert_eq!(compact_field(tests, "failed"), Some("0"));
    assert_eq!(compact_field(tests, "differential"), Some("equal"));
    assert_eq!(
        std::fs::read(project.join("HEAD")).expect("HEAD after check"),
        head_before_check
    );

    let plan = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "rename.owner",
            "--base",
            &initial_revision,
            "--owner",
            &application_owner,
            "--name",
            "application-renamed",
        ],
    );
    let token = compact_field(compact_record(&plan, "plan"), "token")
        .expect("review token")
        .to_owned();
    let changed = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "rename.owner",
            "--base",
            &initial_revision,
            "--owner",
            &application_owner,
            "--name",
            "application-renamed",
            "--plan",
            &token,
        ],
    );
    assert_eq!(compact_field(&changed[0], "status"), Some("accepted"));
    assert_eq!(
        compact_field(compact_record(&changed, "derived-cache"), "status"),
        Some("updated")
    );
    let accepted_revision = compact_field(compact_record(&changed, "revision"), "result")
        .expect("accepted revision")
        .to_owned();
    assert_ne!(accepted_revision, initial_revision);

    let checked_after = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "check"],
    );
    assert_eq!(
        compact_field(compact_record(&checked_after, "compilation"), "cache"),
        Some("exact-current")
    );
    assert_eq!(
        compact_field(compact_record(&checked_after, "tests"), "passed"),
        Some("12")
    );

    let artifact = temporary.path().join("sample.lkja");
    let built = compact_success_at(
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
    assert_eq!(
        compact_field(compact_record(&built, "artifact"), "packages"),
        Some("2")
    );
    assert_eq!(
        compact_field(compact_record(&built, "output"), "visibility"),
        Some("created")
    );
    let artifact_bytes = std::fs::read(&artifact).expect("published artifact");
    let conflict = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "build",
            "--output",
            path(&artifact),
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&conflict, "diagnostic"), "code"),
        Some("output_conflict")
    );
    assert_eq!(
        std::fs::read(&artifact).expect("preserved artifact"),
        artifact_bytes
    );

    let second_artifact = temporary.path().join("sample-second.lkja");
    compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "build",
            "--output",
            path(&second_artifact),
        ],
    );
    assert_eq!(
        std::fs::read(&second_artifact).expect("second artifact"),
        artifact_bytes
    );
    let head_before_run = std::fs::read(project.join("HEAD")).expect("HEAD before run");
    let ran = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "run", "main"],
    );
    let execution = compact_record(&ran, "execution");
    assert_eq!(compact_field(execution, "value"), Some("\"hello\""));
    assert_eq!(compact_field(execution, "differential"), Some("equal"));
    assert_eq!(
        std::fs::read(project.join("HEAD")).expect("HEAD after run"),
        head_before_run
    );
}

#[test]
fn copied_binary_authors_and_runs_a_generic_named_function_value() {
    let temporary = tempfile::TempDir::new().expect("isolated higher-order workspace");
    let copied_binary = temporary.path().join("lkjscript");
    copy_executable(&binary(), &copied_binary);
    let project = temporary.path().join("app");

    let created = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "new",
            path(&project),
            "--template",
            "command",
            "--name",
            "higher-order",
        ],
    );
    let initial_revision = compact_field(compact_record(&created, "revision"), "id")
        .expect("created revision")
        .to_owned();
    let application = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "module",
            "application",
        ],
    );
    let application = compact_field(compact_record(&application, "owner"), "id")
        .expect("application module")
        .to_owned();
    let greet = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "declaration",
            "greet",
            "--parent",
            &application,
        ],
    );
    let greet = compact_field(compact_record(&greet, "owner"), "id")
        .expect("greet function")
        .to_owned();

    let initial_inventory = content_inventory(&project);
    for alias in ["function-ref", "lambda", "apply"] {
        let rejected_path = temporary.path().join(format!("reject-{alias}.lkjc"));
        std::fs::write(
            &rejected_path,
            format!(
                "request base={initial_revision}\n\
                 expression.{alias} as=$body function={greet}\n\
                 create.function as=$rejected module={application} name=rejected visibility=private result=unit effect=pure body=$body\n"
            ),
        )
        .expect("write rejected higher-order alias");
        let rejected = compact_failure_output(command_at(
            &copied_binary,
            temporary.path(),
            &[
                "--project",
                path(&project),
                "change",
                "plan",
                "--input-file",
                path(&rejected_path),
            ],
        ));
        assert!(
            rejected.iter().any(|record| {
                record.operation == "diagnostic"
                    && compact_field(record, "code") == Some("change_expression_form_unknown")
            }),
            "{alias}: {rejected:?}"
        );
        assert_eq!(content_inventory(&project), initial_inventory, "{alias}");
        assert_eq!(
            current_revision_at(&copied_binary, temporary.path(), &project),
            initial_revision,
            "{alias}"
        );
    }

    let rejected_semantics = [
        (
            "generic-task",
            format!(
                "request base={initial_revision}\n\
                 expression.unit as=$task_body\n\
                 create.function as=$task module={application} name=generic-task visibility=private result=unit effect=task body=$task_body\n\
                 add.type-parameter as=$task_type function=$task name=Item\n"
            ),
            "kernel_owner_generic_task",
        ),
        (
            "task-function-value",
            format!(
                "request base={initial_revision}\n\
                 expression.unit as=$task_body\n\
                 create.function as=$task module={application} name=task-value-target visibility=private result=unit effect=task body=$task_body\n\
                 expression.function-value as=$task_value function=$task\n\
                 expression.unit as=$done\n\
                 expression.sequence as=$observer_body\n\
                 expression.argument parent=$observer_body index=0 expression=$task_value\n\
                 expression.argument parent=$observer_body index=1 expression=$done\n\
                 create.function as=$observer module={application} name=task-value-observer visibility=private result=unit effect=pure body=$observer_body\n"
            ),
            "kernel_type_task_function_value",
        ),
        (
            "missing-type-argument",
            format!(
                "request base={initial_revision}\n\
                 type.parameter as=@item parameter=$item_type\n\
                 expression.local as=$identity_body value=$identity_value\n\
                 create.function as=$identity module={application} name=generic-identity visibility=private result=@item effect=pure body=$identity_body\n\
                 add.type-parameter as=$item_type function=$identity name=Item\n\
                 add.parameter as=$identity_value function=$identity name=value type=@item\n\
                 expression.function-value as=$identity_value_expression function=$identity\n\
                 expression.unit as=$done\n\
                 expression.sequence as=$observer_body\n\
                 expression.argument parent=$observer_body index=0 expression=$identity_value_expression\n\
                 expression.argument parent=$observer_body index=1 expression=$done\n\
                 create.function as=$observer module={application} name=missing-type-observer visibility=private result=unit effect=pure body=$observer_body\n"
            ),
            "kernel_type_argument_count",
        ),
        (
            "excess-type-argument",
            format!(
                "request base={initial_revision}\n\
                 expression.unit as=$target_body\n\
                 create.function as=$target module={application} name=non-generic-target visibility=private result=unit effect=pure body=$target_body\n\
                 expression.function-value as=$target_value function=$target\n\
                 type.argument parent=$target_value index=0 type=text\n\
                 expression.unit as=$done\n\
                 expression.sequence as=$observer_body\n\
                 expression.argument parent=$observer_body index=0 expression=$target_value\n\
                 expression.argument parent=$observer_body index=1 expression=$done\n\
                 create.function as=$observer module={application} name=excess-type-observer visibility=private result=unit effect=pure body=$observer_body\n"
            ),
            "kernel_type_argument_count",
        ),
        (
            "invoke-nonfunction",
            format!(
                "request base={initial_revision}\n\
                 expression.text as=$not_function value=text\n\
                 expression.invoke as=$invalid_body function=$not_function\n\
                 create.function as=$invalid module={application} name=invoke-nonfunction visibility=private result=unit effect=pure body=$invalid_body\n"
            ),
            "kernel_type_invoke",
        ),
        (
            "invoke-wrong-arity",
            format!(
                "request base={initial_revision}\n\
                 expression.local as=$target_body value=$target_parameter\n\
                 create.function as=$target module={application} name=arity-target visibility=private result=text effect=pure body=$target_body\n\
                 add.parameter as=$target_parameter function=$target name=value type=text\n\
                 expression.function-value as=$target_value function=$target\n\
                 expression.invoke as=$invalid_body function=$target_value\n\
                 create.function as=$invalid module={application} name=invoke-wrong-arity visibility=private result=text effect=pure body=$invalid_body\n"
            ),
            "kernel_type_call_arity",
        ),
        (
            "invoke-wrong-type",
            format!(
                "request base={initial_revision}\n\
                 expression.local as=$target_body value=$target_parameter\n\
                 create.function as=$target module={application} name=type-target visibility=private result=text effect=pure body=$target_body\n\
                 add.parameter as=$target_parameter function=$target name=value type=text\n\
                 expression.function-value as=$target_value function=$target\n\
                 expression.bool as=$wrong_argument value=true\n\
                 expression.invoke as=$invalid_body function=$target_value\n\
                 expression.argument parent=$invalid_body index=0 expression=$wrong_argument\n\
                 create.function as=$invalid module={application} name=invoke-wrong-type visibility=private result=text effect=pure body=$invalid_body\n"
            ),
            "kernel_type_argument",
        ),
    ];
    for (name, body, expected_code) in rejected_semantics {
        let rejected_path = temporary.path().join(format!("reject-{name}.lkjc"));
        std::fs::write(&rejected_path, body).expect("write rejected higher-order semantics");
        let rejected = compact_failure_output(command_at(
            &copied_binary,
            temporary.path(),
            &[
                "--project",
                path(&project),
                "change",
                "plan",
                "--input-file",
                path(&rejected_path),
            ],
        ));
        assert!(
            rejected.iter().any(|record| {
                record.operation == "diagnostic"
                    && compact_field(record, "code") == Some(expected_code)
            }),
            "{name}: {rejected:?}"
        );
        assert_eq!(content_inventory(&project), initial_inventory, "{name}");
        assert_eq!(
            current_revision_at(&copied_binary, temporary.path(), &project),
            initial_revision,
            "{name}"
        );
    }

    let request = format!(
        "request base={initial_revision} idempotency=public-higher-order-1 intent=author-generic-named-invocation\n\
         type.parameter as=@item parameter=$item_type\n\
         type.function as=@step result=@item\n\
         type.argument parent=@step index=0 type=@item\n\
         expression.local as=$step_local value=$step\n\
         expression.local as=$value_local value=$value\n\
         expression.invoke as=$apply_body function=$step_local\n\
         expression.argument parent=$apply_body index=0 expression=$value_local\n\
         create.function as=$apply module={application} name=apply visibility=private result=@item effect=pure body=$apply_body\n\
         add.type-parameter as=$item_type function=$apply name=Item\n\
         add.parameter as=$value function=$apply name=value type=@item\n\
         add.parameter as=$step function=$apply name=step type=@step\n\
         expression.local as=$keep_body value=$keep_value\n\
         create.function as=$keep module={application} name=keep visibility=private result=text effect=pure body=$keep_body\n\
         add.parameter as=$keep_value function=$keep name=value type=text\n\
         expression.function-value as=$apply_value function=$apply\n\
         type.argument parent=$apply_value index=0 type=text\n\
         expression.function-value as=$keep_value_expression function=$keep\n\
         expression.text as=$text value=hello\n\
         expression.invoke as=$greet_body function=$apply_value\n\
         expression.argument parent=$greet_body index=0 expression=$text\n\
         expression.argument parent=$greet_body index=1 expression=$keep_value_expression\n\
         replace.body function={greet} body=$greet_body\n"
    );
    let request_path = temporary.path().join("higher-order.lkjc");
    std::fs::write(&request_path, request).expect("write higher-order compact request");
    let plan_arguments = [
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&request_path),
    ];
    let planned = compact_success_at(&copied_binary, temporary.path(), &plan_arguments);
    let plan = compact_field(compact_record(&planned, "plan"), "token")
        .expect("higher-order review token")
        .to_owned();
    let planned_again = compact_success_at(&copied_binary, temporary.path(), &plan_arguments);
    assert_eq!(
        compact_field(compact_record(&planned_again, "plan"), "token"),
        Some(plan.as_str())
    );
    assert_eq!(content_inventory(&project), initial_inventory);
    let symbols = planned
        .iter()
        .filter(|record| record.operation == "identity")
        .filter_map(|record| compact_field(record, "symbol"))
        .collect::<BTreeSet<_>>();
    for expected in [
        "$apply",
        "$item_type",
        "$step",
        "$apply_value",
        "$keep_value_expression",
        "$greet_body",
    ] {
        assert!(
            symbols.contains(expected),
            "missing planned identity {expected}"
        );
    }

    let applied = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "--input-file",
            path(&request_path),
            "--plan",
            &plan,
        ],
    );
    assert_eq!(compact_field(&applied[0], "status"), Some("accepted"));
    let accepted = compact_field(compact_record(&applied, "revision"), "result")
        .expect("accepted higher-order revision")
        .to_owned();
    assert_ne!(accepted, initial_revision);

    let applied_function = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "declaration",
            "apply",
            "--parent",
            &application,
        ],
    );
    let applied_function = compact_field(compact_record(&applied_function, "owner"), "id")
        .expect("accepted generic function")
        .to_owned();
    let inspected = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "inspect",
            "owner",
            "pure_function",
            &applied_function,
        ],
    );
    assert_eq!(
        compact_field(compact_record(&inspected, "revision"), "observed"),
        Some(accepted.as_str())
    );
    assert_eq!(
        compact_field(compact_record(&inspected, "owner"), "name"),
        Some("apply")
    );

    let checked = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "check"],
    );
    assert_eq!(
        compact_field(compact_record(&checked, "compilation"), "cache"),
        Some("clean")
    );
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "failed"),
        Some("0")
    );
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "differential"),
        Some("equal")
    );
    let checked_again = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "check"],
    );
    assert_eq!(
        compact_field(compact_record(&checked_again, "compilation"), "cache"),
        Some("exact-current")
    );
    assert_eq!(
        compact_field(compact_record(&checked_again, "tests"), "differential"),
        Some("equal")
    );

    let first_artifact = temporary.path().join("higher-order.lkja");
    let second_artifact = temporary.path().join("higher-order-second.lkja");
    for output in [&first_artifact, &second_artifact] {
        compact_success_at(
            &copied_binary,
            temporary.path(),
            &[
                "--project",
                path(&project),
                "build",
                "--output",
                path(output),
            ],
        );
    }
    assert_eq!(
        std::fs::read(&first_artifact).expect("first higher-order artifact"),
        std::fs::read(&second_artifact).expect("second higher-order artifact")
    );
    let before_run_head =
        std::fs::read(project.join("HEAD")).expect("HEAD before higher-order run");
    let ran = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "run", "main"],
    );
    let execution = compact_record(&ran, "execution");
    assert_eq!(compact_field(execution, "value"), Some("\"hello\""));
    assert_eq!(compact_field(execution, "differential"), Some("equal"));
    assert_eq!(
        std::fs::read(project.join("HEAD")).expect("HEAD after higher-order run"),
        before_run_head
    );
    assert_eq!(
        current_revision_at(&copied_binary, temporary.path(), &project),
        accepted
    );
}

#[test]
fn copied_binary_fold_exhaustion_is_typed_and_preserves_authority() {
    const ITEMS: usize = 4_096;

    let temporary = tempfile::TempDir::new().expect("isolated fold exhaustion workspace");
    let copied_binary = temporary.path().join("lkjscript");
    copy_executable(&binary(), &copied_binary);
    let project = temporary.path().join("app");
    let created = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "new",
            path(&project),
            "--template",
            "command",
            "--name",
            "fold-exhaustion",
        ],
    );
    let initial_revision = compact_field(compact_record(&created, "revision"), "id")
        .expect("created revision")
        .to_owned();
    let application = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "module",
            "application",
        ],
    );
    let application = compact_field(compact_record(&application, "owner"), "id")
        .expect("application module")
        .to_owned();
    let greet = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "declaration",
            "greet",
            "--parent",
            &application,
        ],
    );
    let greet = compact_field(compact_record(&greet, "owner"), "id")
        .expect("greet function")
        .to_owned();
    let fold = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "package",
            "builtin",
            "query",
            "owners",
            "--name",
            "list-fold-left",
        ],
    );
    let fold = compact_field(compact_record(&fold, "owner"), "reference")
        .expect("built-in fold reference")
        .to_owned();

    let mut request = format!(
        "request base={initial_revision} idempotency=fold-exhaustion-1 intent=prove-typed-fold-call-depth-admission\n\
         expression.local as=$step_body value=$step_state\n\
         create.function as=$step module={application} name=retain-fold-state visibility=private result=text effect=pure body=$step_body\n\
         add.parameter as=$step_state function=$step name=state type=text\n\
         add.parameter as=$step_item function=$step name=item type=i64\n\
         expression.function-value as=$step_value function=$step\n\
         expression.text as=$initial value=hello\n\
         expression.list as=$items item=i64\n"
    );
    for index in 0..ITEMS {
        request.push_str(&format!(
            "expression.i64 as=$item{index:04} value={index}\n\
             expression.argument parent=$items index={index} expression=$item{index:04}\n"
        ));
    }
    request.push_str(&format!(
        "expression.call as=$greet_body function={fold}\n\
         type.argument parent=$greet_body index=0 type=i64\n\
         type.argument parent=$greet_body index=1 type=text\n\
         expression.argument parent=$greet_body index=0 expression=$items\n\
         expression.argument parent=$greet_body index=1 expression=$initial\n\
         expression.argument parent=$greet_body index=2 expression=$step_value\n\
         replace.body function={greet} body=$greet_body\n"
    ));
    let request_path = temporary.path().join("fold-exhaustion.lkjc");
    std::fs::write(&request_path, request).expect("write fold exhaustion request");
    let logical_plan = temporary.path().join("fold-exhaustion.logical-plan");
    let planned_output = command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "--input-file",
            path(&request_path),
            "--output",
            path(&logical_plan),
        ],
    );
    assert!(
        planned_output.status.success(),
        "fold plan failed: {}",
        String::from_utf8_lossy(&planned_output.stdout)
    );
    assert!(planned_output.stderr.is_empty());
    assert!(planned_output.stdout.len() < 2 * 1_048_576);
    let planned = parse_records("fold exhaustion plan", &planned_output.stdout)
        .expect("fold exhaustion plan records");
    let plan = compact_field(compact_record(&planned, "plan"), "token")
        .expect("fold exhaustion plan token")
        .to_owned();
    let decoded = decode_logical_change_plan(BufReader::new(
        File::open(&logical_plan).expect("open fold exhaustion logical plan"),
    ))
    .expect("decode fold exhaustion logical plan");
    assert_eq!(decoded.token, plan);
    assert!(decoded.counts.allocations > ITEMS as u64);

    let applied_output = command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "--input-file",
            path(&request_path),
            "--plan",
            &plan,
        ],
    );
    assert!(
        applied_output.status.success(),
        "fold apply failed: {}",
        String::from_utf8_lossy(&applied_output.stdout)
    );
    assert!(applied_output.stderr.is_empty());
    assert!(applied_output.stdout.len() < 2 * 1_048_576);
    let applied = parse_records("fold exhaustion apply", &applied_output.stdout)
        .expect("fold exhaustion apply records");
    assert_eq!(compact_field(&applied[0], "status"), Some("accepted"));
    let accepted = compact_field(compact_record(&applied, "revision"), "result")
        .expect("accepted fold exhaustion revision")
        .to_owned();
    let before_run_head = std::fs::read(project.join("HEAD")).expect("HEAD before fold exhaustion");

    let exhausted = compact_failure_output_with_status(
        command_at(
            &copied_binary,
            temporary.path(),
            &["--project", path(&project), "run", "main"],
        ),
        4,
    );
    assert_eq!(
        compact_field(compact_record(&exhausted, "diagnostic"), "class"),
        Some("resource")
    );
    assert_eq!(
        compact_field(compact_record(&exhausted, "diagnostic"), "code"),
        Some("normalized_call_depth")
    );
    assert_eq!(
        std::fs::read(project.join("HEAD")).expect("HEAD after fold exhaustion"),
        before_run_head
    );
    assert_eq!(
        current_revision_at(&copied_binary, temporary.path(), &project),
        accepted
    );
}

#[test]
fn corrupt_derived_cache_recovers_without_changing_semantic_publication_outcomes() {
    let temporary = tempfile::TempDir::new().expect("isolated binary workspace");
    let copied_binary = temporary.path().join("lkjscript");
    copy_executable(&binary(), &copied_binary);
    let project = temporary.path().join("app");

    let created = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "new",
            path(&project),
            "--template",
            "command",
            "--name",
            "cache-recovery",
        ],
    );
    let initial_revision = compact_field(compact_record(&created, "revision"), "id")
        .expect("created revision")
        .to_owned();
    compact_success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "check"],
    );
    let cache_head = project.join("derived/compiler/CURRENT");
    std::fs::write(&cache_head, b"broken").expect("corrupt derived cache head");
    let semantic_head = std::fs::read(project.join("HEAD")).expect("semantic HEAD before recovery");

    let recovered = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "check"],
    );
    let compilation = compact_record(&recovered, "compilation");
    assert_eq!(compact_field(compilation, "cache"), Some("clean-recovery"));
    assert_eq!(
        compact_field(compilation, "recovered-class"),
        Some("corrupt")
    );
    assert_eq!(
        compact_field(compilation, "recovered-code"),
        Some("packed_truncated")
    );
    assert_eq!(
        std::fs::read(project.join("HEAD")).expect("semantic HEAD after recovery"),
        semantic_head
    );

    let application = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "module",
            "application",
        ],
    );
    let application_owner = compact_field(compact_record(&application, "owner"), "id")
        .expect("application module")
        .to_owned();
    let plan = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "rename.owner",
            "--base",
            &initial_revision,
            "--owner",
            &application_owner,
            "--name",
            "application-renamed",
        ],
    );
    let token = compact_field(compact_record(&plan, "plan"), "token")
        .expect("review token")
        .to_owned();
    std::fs::write(&cache_head, b"broken-again").expect("recorrupt derived cache head");
    let changed = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "rename.owner",
            "--base",
            &initial_revision,
            "--owner",
            &application_owner,
            "--name",
            "application-renamed",
            "--plan",
            &token,
        ],
    );
    assert_eq!(compact_field(&changed[0], "status"), Some("accepted"));
    let derived_cache = compact_record(&changed, "derived-cache");
    assert_eq!(compact_field(derived_cache, "status"), Some("failed"));
    assert_eq!(
        compact_field(derived_cache, "diagnostic-class"),
        Some("corrupt")
    );
    assert_eq!(
        compact_field(derived_cache, "diagnostic-code"),
        Some("packed_truncated")
    );
    let accepted_revision =
        compact_field(compact_record(&changed, "revision"), "result").expect("accepted revision");
    assert_ne!(accepted_revision, initial_revision);

    let post_accept_recovery = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "check"],
    );
    assert_eq!(
        compact_field(
            compact_record(&post_accept_recovery, "compilation"),
            "cache"
        ),
        Some("clean-recovery")
    );
    assert_eq!(
        compact_field(compact_record(&post_accept_recovery, "tests"), "failed"),
        Some("0")
    );
}

#[test]
fn reviewed_change_plan_body_replacement_exports_exact_owned_relation_closure() {
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
         add.field as=$message-spare record=$message name=spare type=unit\n\
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
    let plan = compact_field(compact_record(&planned, "plan"), "token")
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
    assert_eq!(planned_identities.len(), 8);

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

    let record = planned_identities
        .iter()
        .find_map(|(symbol, identity)| (*symbol == "$message").then_some(*identity))
        .expect("allocated record identity");
    let cascade_request = format!(
        "request base={accepted_revision}\n\
         delete.owner owner={record} cascade=true policy=reject\n"
    );
    let cascade_request_path = temporary.path().join("reject-delete-cascade.lkjc");
    std::fs::write(&cascade_request_path, cascade_request).expect("unsupported deletion request");
    let rejected = compact_failure_output(command(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&cascade_request_path),
    ]));
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some("change_field_unknown")
    );
    let after_rejection = compact_success(&["--project", path(&project), "status"]);
    assert_eq!(
        compact_field(compact_record(&after_rejection, "revision"), "id"),
        Some(accepted_revision.as_str())
    );
    let rejected_delete = format!(
        "request base={accepted_revision}\n\
         delete.owner owner={record} policy=reject\n"
    );
    let rejected_delete_path = temporary.path().join("reject-owned-delete.lkjc");
    std::fs::write(&rejected_delete_path, rejected_delete)
        .expect("owned deletion rejection request");
    let rejected = compact_failure_output(command(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&rejected_delete_path),
    ]));
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some("change_delete_owned_children")
    );
    let after_rejection = compact_success(&["--project", path(&project), "status"]);
    assert_eq!(
        compact_field(compact_record(&after_rejection, "revision"), "id"),
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
        "plan_00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    ]));
    assert_eq!(
        compact_field(compact_record(&mismatch, "diagnostic"), "code"),
        Some("change_request_commitment_mismatch")
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
    let replacement_output = temporary.path().join("replacement.logical-plan");
    let before_replacement_plan = content_inventory(&project);
    let replacement_plan = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&replacement_path),
        "--output",
        path(&replacement_output),
    ]);
    assert_eq!(
        compact_field(compact_record(&replacement_plan, "summary"), "updated"),
        Some("1")
    );
    assert_eq!(
        compact_field(compact_record(&replacement_plan, "summary"), "deleted"),
        Some("2")
    );
    let replacement_digest = compact_field(compact_record(&replacement_plan, "plan"), "token")
        .expect("replacement plan digest");
    assert_eq!(content_inventory(&project), before_replacement_plan);
    assert_eq!(current_revision(&project), accepted_revision);
    let decoded = decode_logical_change_plan(BufReader::new(
        File::open(&replacement_output).expect("open replacement logical plan"),
    ))
    .expect("decode replacement logical plan");
    assert_eq!(decoded.token, replacement_digest);
    let replacement_records = parse_records(
        "replacement logical plan",
        &std::fs::read(&replacement_output).expect("read replacement logical plan"),
    )
    .expect("parse replacement logical plan records");
    let plan_header = compact_record(&replacement_records, "logical-plan");
    assert_eq!(compact_field(plan_header, "product"), Some("lkjscript"));
    assert_eq!(
        compact_field(plan_header, "version"),
        Some(lkjscript::PRODUCT_VERSION)
    );
    assert_eq!(
        compact_field(
            compact_record(&replacement_records, "logical-plan.capabilities"),
            "digest"
        )
        .expect("logical plan capabilities digest")
        .len(),
        64
    );
    assert!(
        !replacement_records
            .iter()
            .any(|record| record.operation == "logical-plan.contracts")
    );
    let plan_text = std::fs::read_to_string(&replacement_output).expect("logical plan UTF-8");
    for forbidden in [
        "lkjscript-logical-change-plan-",
        "lkjscript-meaning-graph-",
        "lkjscript-change-records-",
        "lkjscript-authored-change-codec-",
        "logical-plan.contracts",
        " contract=",
    ] {
        assert!(
            !plan_text.contains(forbidden),
            "logical plan leaked {forbidden}"
        );
    }
    let old_read = planned_identities
        .iter()
        .find_map(|(symbol, identity)| (*symbol == "$read").then_some(*identity))
        .expect("old local-reference expression");
    let old_body = planned_identities
        .iter()
        .find_map(|(symbol, identity)| (*symbol == "$body").then_some(*identity))
        .expect("old sequence body expression");
    let parameter = planned_identities
        .iter()
        .find_map(|(symbol, identity)| (*symbol == "$value").then_some(*identity))
        .expect("function parameter");
    let replacement_owner = replacement_plan
        .iter()
        .find(|record| {
            record.operation == "identity"
                && compact_field(record, "symbol") == Some("$replacement")
        })
        .and_then(|record| compact_field(record, "id"))
        .expect("replacement expression identity");
    let deleted_owners = replacement_records
        .iter()
        .filter(|record| {
            record.operation == "logical-plan.owner"
                && compact_field(record, "class-deleted") == Some("true")
        })
        .filter_map(|record| compact_field(record, "owner"))
        .collect::<BTreeSet<_>>();
    assert_eq!(deleted_owners, BTreeSet::from([old_read, old_body]));
    let created_owners = replacement_records
        .iter()
        .filter(|record| {
            record.operation == "logical-plan.owner"
                && compact_field(record, "class-created") == Some("true")
        })
        .filter_map(|record| compact_field(record, "owner"))
        .collect::<BTreeSet<_>>();
    assert_eq!(created_owners, BTreeSet::from([replacement_owner]));
    let retired_owners = replacement_records
        .iter()
        .filter(|record| {
            record.operation == "logical-plan.retirement"
                && compact_field(record, "before-present") == Some("false")
                && compact_field(record, "after-present") == Some("true")
        })
        .filter_map(|record| compact_field(record, "owner"))
        .collect::<BTreeSet<_>>();
    assert_eq!(retired_owners, BTreeSet::from([old_read, old_body]));
    let removed_relations = replacement_records
        .iter()
        .filter(|record| record.operation == "logical-plan.relation-removed")
        .map(|record| {
            (
                compact_field(record, "source-owner").unwrap(),
                compact_field(record, "kind").unwrap(),
                compact_field(record, "target-owner").unwrap(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        removed_relations,
        BTreeSet::from([
            (old_read, "expression_parent", old_body),
            (old_read, "local_value_reference", parameter),
            (old_body, "expression_root", function),
        ])
    );
    let added_relations = replacement_records
        .iter()
        .filter(|record| record.operation == "logical-plan.relation-added")
        .map(|record| {
            (
                compact_field(record, "source-owner").unwrap(),
                compact_field(record, "kind").unwrap(),
                compact_field(record, "target-owner").unwrap(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        added_relations,
        BTreeSet::from([(replacement_owner, "expression_root", function)])
    );

    let presentation_only_request = format!(
        "request base={accepted_revision}\n\
         expression.text as=$different-local-label value=replaced\n\
         replace.body function={function} body=$different-local-label\n"
    );
    let presentation_output = temporary.path().join("replacement-renamed.logical-plan");
    let presentation_plan = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input",
        &presentation_only_request,
        "--output",
        path(&presentation_output),
    ]);
    assert_eq!(
        compact_field(compact_record(&presentation_plan, "plan"), "token"),
        Some(replacement_digest)
    );
    assert_eq!(
        std::fs::read(&presentation_output).expect("renamed-label logical plan"),
        std::fs::read(&replacement_output).expect("original-label logical plan")
    );
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

    let replaced_revision = compact_field(compact_record(&replaced, "revision"), "result")
        .expect("replacement revision");
    let field = planned_identities
        .iter()
        .find_map(|(symbol, identity)| (*symbol == "$message-text").then_some(*identity))
        .expect("allocated field identity");
    for (name, request, code) in [
        (
            "predecessor-precondition",
            format!(
                "request base={replaced_revision}\n\
                 precondition.semantic-root equals=old\n\
                 delete.owner owner={field} policy=reject\n"
            ),
            "change_precondition_unknown",
        ),
        (
            "failed-semantic-precondition",
            format!(
                "request base={replaced_revision}\n\
                 precondition.owner-name owner={field} name=wrong\n\
                 delete.owner owner={field} policy=reject\n"
            ),
            "change_precondition_owner_name",
        ),
    ] {
        let request_path = temporary.path().join(format!("{name}.lkjc"));
        std::fs::write(&request_path, request).expect("rejected precondition request");
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
            Some(code)
        );
        let after_rejection = compact_success(&["--project", path(&project), "status"]);
        assert_eq!(
            compact_field(compact_record(&after_rejection, "revision"), "id"),
            Some(replaced_revision)
        );
    }
    let deletion = format!(
        "request base={replaced_revision}\n\
         precondition.owner-exists owner={field}\n\
         precondition.owner-name owner={field} name=text\n\
         delete.owner owner={field} policy=reject\n"
    );
    let deletion_path = temporary.path().join("delete.lkjc");
    std::fs::write(&deletion_path, deletion).expect("exact leaf deletion request");
    let deletion_plan = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&deletion_path),
    ]);
    assert_eq!(
        compact_field(compact_record(&deletion_plan, "summary"), "deleted"),
        Some("1")
    );
    assert_eq!(
        compact_field(compact_record(&deletion_plan, "summary"), "updated"),
        Some("1")
    );
    let deletion_digest = compact_field(compact_record(&deletion_plan, "plan"), "token")
        .expect("deletion plan digest");
    let deleted = compact_success(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "--input-file",
        path(&deletion_path),
        "--plan",
        deletion_digest,
    ]);
    assert_eq!(compact_field(&deleted[0], "status"), Some("accepted"));
    assert_eq!(
        compact_field(compact_record(&deleted, "summary"), "deleted"),
        Some("1")
    );
}

#[test]
fn reviewed_change_plan_direct_and_record_export_match_and_apply_reprepares() {
    let temporary = tempfile::TempDir::new().expect("temporary direct rename authority");
    let project = temporary.path().join("project");
    let created = compact_success(&["new", path(&project), "--name", "project"]);
    let initial =
        compact_field(compact_record(&created, "revision"), "id").expect("initial revision");
    let creation = format!("request base={initial}\ncreate.module as=$module name=before\n");
    let creation_plan = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input",
        &creation,
    ]);
    let creation_digest =
        compact_field(compact_record(&creation_plan, "plan"), "token").expect("creation plan");
    let created_module = compact_success(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "--input",
        &creation,
        "--plan",
        creation_digest,
    ]);
    let owner = compact_field(
        created_module
            .iter()
            .find(|record| record.operation == "identity")
            .expect("allocated module"),
        "id",
    )
    .expect("module identity")
    .to_owned();
    let base = compact_field(compact_record(&created_module, "revision"), "result")
        .expect("rename base")
        .to_owned();

    let record_without_controls =
        format!("request base={base}\nrename.owner owner={owner} name=renamed\n");
    let record_output = command(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input",
        &record_without_controls,
    ]);
    let direct_output = command(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "rename.owner",
        "--base",
        &base,
        "--owner",
        &owner,
        "--name",
        "renamed",
    ]);
    assert_eq!(record_output.status, direct_output.status);
    assert_eq!(record_output.stderr, direct_output.stderr);
    assert_eq!(record_output.stdout, direct_output.stdout);
    let without_controls = compact_success_output(record_output);
    let plan_without_controls = compact_field(compact_record(&without_controls, "plan"), "token")
        .expect("plan without controls")
        .to_owned();

    let record_with_controls = format!(
        "request base={base} idempotency=direct-rename-equality intent=transport-equality\n\
         rename.owner owner={owner} name=renamed\n"
    );
    let record_output = command(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input",
        &record_with_controls,
    ]);
    let direct_output = command(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "rename.owner",
        "--base",
        &base,
        "--owner",
        &owner,
        "--name",
        "renamed",
        "--idempotency",
        "direct-rename-equality",
        "--intent",
        "transport-equality",
    ]);
    assert_eq!(record_output.status, direct_output.status);
    assert_eq!(record_output.stderr, direct_output.stderr);
    assert_eq!(record_output.stdout, direct_output.stdout);
    let planned = compact_success_output(direct_output);
    let plan = compact_field(compact_record(&planned, "plan"), "token")
        .expect("reviewed rename plan")
        .to_owned();
    assert_ne!(plan, plan_without_controls);

    let before_plan_inventory = content_inventory(&project);
    let record_plan_path = temporary.path().join("record.logical-plan");
    let direct_plan_path = temporary.path().join("direct.logical-plan");
    let record_export = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input",
        &record_with_controls,
        "--output",
        path(&record_plan_path),
    ]);
    let direct_export = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "rename.owner",
        "--base",
        &base,
        "--owner",
        &owner,
        "--name",
        "renamed",
        "--idempotency",
        "direct-rename-equality",
        "--intent",
        "transport-equality",
        "--output",
        path(&direct_plan_path),
    ]);
    assert_eq!(
        compact_field(compact_record(&record_export, "plan"), "token"),
        Some(plan.as_str())
    );
    assert_eq!(
        compact_field(compact_record(&direct_export, "plan"), "token"),
        Some(plan.as_str())
    );
    assert_eq!(
        compact_field(compact_record(&record_export, "plan-output"), "status"),
        Some("published")
    );
    assert_eq!(
        std::fs::read(&record_plan_path).expect("record plan file"),
        std::fs::read(&direct_plan_path).expect("direct plan file")
    );
    let decoded = decode_logical_change_plan(BufReader::new(
        File::open(&record_plan_path).expect("open logical plan"),
    ))
    .expect("strict logical plan decode");
    assert_eq!(decoded.token, plan);
    assert_eq!(
        decoded.bytes.to_string(),
        compact_field(compact_record(&record_export, "plan-output"), "bytes").unwrap()
    );
    assert_eq!(
        decoded.records.to_string(),
        compact_field(compact_record(&record_export, "plan-output"), "records").unwrap()
    );
    let unchanged_export = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input",
        &record_with_controls,
        "--output",
        path(&record_plan_path),
    ]);
    assert_eq!(
        compact_field(compact_record(&unchanged_export, "plan-output"), "status"),
        Some("unchanged")
    );
    assert_eq!(current_revision(&project), base);
    assert_eq!(content_inventory(&project), before_plan_inventory);

    std::fs::write(&record_plan_path, b"noncanonical old output")
        .expect("replace review output fixture");
    let republished = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input",
        &record_with_controls,
        "--output",
        path(&record_plan_path),
    ]);
    assert_eq!(
        compact_field(compact_record(&republished, "plan-output"), "status"),
        Some("published")
    );
    decode_logical_change_plan(BufReader::new(
        File::open(&record_plan_path).expect("open republished logical plan"),
    ))
    .expect("republished logical plan is complete");

    let inside_project = project.join("review.logical-plan");
    let rejected_inside = compact_failure_output(command(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input",
        &record_with_controls,
        "--output",
        path(&inside_project),
    ]));
    assert_eq!(
        compact_field(compact_record(&rejected_inside, "diagnostic"), "code"),
        Some("change_plan_output_project_path")
    );
    assert!(!inside_project.exists());

    let directory_target = temporary.path().join("review-directory");
    std::fs::create_dir(&directory_target).expect("plan output directory fixture");
    let rejected_directory = compact_failure_output(command(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input",
        &record_with_controls,
        "--output",
        path(&directory_target),
    ]));
    assert_eq!(
        compact_field(compact_record(&rejected_directory, "diagnostic"), "code"),
        Some("change_plan_output_type")
    );

    let missing_parent = temporary.path().join("absent/review.logical-plan");
    let rejected_parent = compact_failure_output_with_status(
        command(&[
            "--project",
            path(&project),
            "change",
            "plan",
            "--input",
            &record_with_controls,
            "--output",
            path(&missing_parent),
        ]),
        6,
    );
    assert_eq!(
        compact_field(compact_record(&rejected_parent, "diagnostic"), "code"),
        Some("change_plan_output_parent")
    );
    assert!(!missing_parent.exists());

    #[cfg(unix)]
    {
        let protected = temporary.path().join("protected-output");
        let symlink = temporary.path().join("review-symlink");
        std::fs::write(&protected, b"protected").expect("protected output fixture");
        std::os::unix::fs::symlink(&protected, &symlink).expect("plan output symlink fixture");
        let rejected_symlink = compact_failure_output(command(&[
            "--project",
            path(&project),
            "change",
            "plan",
            "--input",
            &record_with_controls,
            "--output",
            path(&symlink),
        ]));
        assert_eq!(
            compact_field(compact_record(&rejected_symlink, "diagnostic"), "code"),
            Some("change_plan_output_type")
        );
        assert_eq!(
            std::fs::read(&protected).expect("protected output remains"),
            b"protected"
        );
    }
    assert_eq!(current_revision(&project), base);
    assert_eq!(content_inventory(&project), before_plan_inventory);

    let mut wrong_prepared = plan.clone();
    let replacement = if wrong_prepared.ends_with('0') {
        '1'
    } else {
        '0'
    };
    wrong_prepared.pop();
    wrong_prepared.push(replacement);
    let rejected = compact_failure_output(command(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "rename.owner",
        "--base",
        &base,
        "--owner",
        &owner,
        "--name",
        "renamed",
        "--idempotency",
        "direct-rename-equality",
        "--intent",
        "transport-equality",
        "--plan",
        &wrong_prepared,
    ]));
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some("change_prepared_plan_mismatch")
    );
    assert_eq!(current_revision(&project), base);

    let applied = compact_success(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "rename.owner",
        "--base",
        &base,
        "--owner",
        &owner,
        "--name",
        "renamed",
        "--idempotency",
        "direct-rename-equality",
        "--intent",
        "transport-equality",
        "--plan",
        &plan,
    ]);
    assert_eq!(compact_field(&applied[0], "status"), Some("accepted"));
    for operation in [
        "revision",
        "plan",
        "change",
        "summary",
        "validation",
        "receipt",
    ] {
        assert_eq!(
            compact_record_values(compact_record(&planned, operation)),
            compact_record_values(compact_record(&applied, operation)),
            "plan/apply {operation} projection"
        );
    }
    let accepted = compact_field(compact_record(&applied, "revision"), "result")
        .expect("accepted rename revision")
        .to_owned();
    assert_eq!(current_revision(&project), accepted);
    let inspected = compact_success(&[
        "--project",
        path(&project),
        "inspect",
        "owner",
        "module",
        &owner,
    ]);
    assert_eq!(
        compact_field(compact_record(&inspected, "owner"), "name"),
        Some("renamed")
    );

    let repeated = compact_success(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "rename.owner",
        "--base",
        &base,
        "--owner",
        &owner,
        "--name",
        "renamed",
        "--idempotency",
        "direct-rename-equality",
        "--intent",
        "transport-equality",
        "--plan",
        &plan,
    ]);
    assert_eq!(
        compact_field(compact_record(&repeated, "result"), "status"),
        Some("already-accepted")
    );
    assert_eq!(
        compact_field(compact_record(&repeated, "revision"), "result"),
        Some(accepted.as_str())
    );
    assert_eq!(current_revision(&project), accepted);
}

#[test]
#[ignore = "release-scale fixture; run explicitly with --ignored --nocapture"]
fn reviewed_change_plan_scale_streams_detail_beyond_stdout_envelope() {
    const BACKGROUND_MODULES: usize = 500;
    const PAYLOAD_TYPES: usize = 4_500;
    const PRIMITIVES: [&str; 7] = [
        "unit",
        "bool",
        "i64",
        "bytes",
        "text",
        "static-text",
        "secret",
    ];

    let temporary = tempfile::TempDir::new().expect("temporary logical-plan scale authority");
    let project = temporary.path().join("project");
    let created = compact_success(&["new", path(&project), "--name", "plan-scale"]);
    let base = compact_field(compact_record(&created, "revision"), "id")
        .expect("scale base revision")
        .to_owned();

    let mut request = format!("request base={base}\n");
    let mut base_types = Vec::new();
    for form in ["list", "option", "stream"] {
        for primitive in PRIMITIVES {
            let label = format!("@base-{:03}", base_types.len());
            request.push_str(&format!("type.{form} as={label} item={primitive}\n"));
            base_types.push(label);
        }
    }
    for form in ["map", "result"] {
        for left in PRIMITIVES {
            for right in PRIMITIVES {
                let label = format!("@base-{:03}", base_types.len());
                let fields = if form == "map" {
                    format!("key={left} value={right}")
                } else {
                    format!("ok={left} error={right}")
                };
                request.push_str(&format!("type.{form} as={label} {fields}\n"));
                base_types.push(label);
            }
        }
    }
    let mut level = Vec::new();
    for ordinal in 0..PAYLOAD_TYPES {
        let label = format!("@payload-{ordinal:04}");
        let left = &base_types[ordinal % base_types.len()];
        let right = &base_types[(ordinal / base_types.len()) % base_types.len()];
        request.push_str(&format!("type.map as={label} key={left} value={right}\n"));
        level.push(label);
    }
    let mut join_ordinal = 0_usize;
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            if let [left, right] = pair {
                let label = format!("@join-{join_ordinal:04}");
                request.push_str(&format!("type.result as={label} ok={left} error={right}\n"));
                next.push(label);
                join_ordinal += 1;
            } else {
                next.push(pair[0].clone());
            }
        }
        level = next;
    }
    let root_type = level.pop().expect("scale type root");
    for ordinal in 0..BACKGROUND_MODULES {
        request.push_str(&format!(
            "create.module as=$module-{ordinal:03} name=module_{ordinal:03}\n"
        ));
    }
    request.push_str(&format!(
        "create.record as=$record module=$module-000 name=Payload visibility=public\n\
         add.field as=$field record=$record name=value type={root_type}\n"
    ));
    let request_path = temporary.path().join("scale.lkjc");
    std::fs::write(&request_path, request).expect("scale request");
    let before = content_inventory(&project);

    let disabled_started = std::time::Instant::now();
    let disabled = command(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&request_path),
    ]);
    let disabled_elapsed = disabled_started.elapsed();
    assert!(
        disabled.status.success(),
        "scale plan without export failed: {}",
        String::from_utf8_lossy(&disabled.stdout)
    );
    assert!(disabled.stderr.is_empty());
    let disabled_records =
        parse_records("scale plan stdout", &disabled.stdout).expect("scale plan stdout records");
    assert!(disabled_records.len() < 10_000);
    assert_eq!(
        disabled_records
            .iter()
            .filter(|record| record.operation == "identity")
            .count(),
        BACKGROUND_MODULES + 2
    );
    let token = compact_field(compact_record(&disabled_records, "plan"), "token")
        .expect("scale plan token")
        .to_owned();
    assert_eq!(content_inventory(&project), before);

    let plan_path = temporary.path().join("scale.logical-plan");
    let enabled_started = std::time::Instant::now();
    let enabled = command(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&request_path),
        "--output",
        path(&plan_path),
    ]);
    let enabled_elapsed = enabled_started.elapsed();
    assert!(
        enabled.status.success(),
        "scale plan with export failed: {}",
        String::from_utf8_lossy(&enabled.stdout)
    );
    assert!(enabled.stderr.is_empty());
    let enabled_records =
        parse_records("scale export stdout", &enabled.stdout).expect("scale export stdout records");
    assert!(enabled_records.len() < 10_000);
    assert_eq!(
        compact_field(compact_record(&enabled_records, "plan"), "token"),
        Some(token.as_str())
    );
    let decoded = decode_logical_change_plan(BufReader::new(
        File::open(&plan_path).expect("open scale logical plan"),
    ))
    .expect("strict scale logical-plan decode");
    assert_eq!(decoded.token, token);
    assert_eq!(decoded.counts.allocations, (BACKGROUND_MODULES + 2) as u64);
    assert_eq!(decoded.counts.owners, (BACKGROUND_MODULES + 2) as u64);
    assert!(decoded.bytes > enabled.stdout.len() as u64);
    assert_eq!(content_inventory(&project), before);
    assert_eq!(current_revision(&project), base);

    let peak_rss_kib = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("VmHWM:")
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
            })
        });
    eprintln!(
        "reviewed-change-plan-scale background_modules={BACKGROUND_MODULES} payload_types={PAYLOAD_TYPES} join_types={join_ordinal} plan_types={} plan_records={} plan_bytes={} stdout_records={} stdout_bytes={} disabled_wall_us={} enabled_wall_us={} peak_rss_kib={} repository_bytes_written=0",
        decoded.counts.types,
        decoded.records,
        decoded.bytes,
        enabled_records.len(),
        enabled.stdout.len(),
        disabled_elapsed.as_micros(),
        enabled_elapsed.as_micros(),
        peak_rss_kib.map_or_else(|| "unavailable".to_owned(), |value| value.to_string()),
    );
    assert!(
        decoded.records > 10_000,
        "scale logical plan emitted only {} records",
        decoded.records
    );
    assert!(
        decoded.counts.types >= (PAYLOAD_TYPES * 2) as u64,
        "scale logical plan retained only {} type additions",
        decoded.counts.types
    );
    if let Some(evidence_directory) =
        std::env::var_os("LKJSCRIPT_REVIEW_PLAN_EVIDENCE_DIR").map(PathBuf::from)
    {
        std::fs::create_dir_all(&evidence_directory).expect("create retained scale evidence");
        std::fs::copy(&request_path, evidence_directory.join("scale-request.lkjc"))
            .expect("retain scale request");
        std::fs::copy(&plan_path, evidence_directory.join("scale.logical-plan"))
            .expect("retain scale logical plan");
        std::fs::write(
            evidence_directory.join("scale-without-export.stdout"),
            &disabled.stdout,
        )
        .expect("retain scale stdout without export");
        std::fs::write(
            evidence_directory.join("scale-with-export.stdout"),
            &enabled.stdout,
        )
        .expect("retain scale stdout with export");
    }
}

#[test]
fn direct_rename_malformed_inputs_and_plan_mismatch_never_access_or_advance_authority() {
    let temporary = tempfile::TempDir::new().expect("temporary direct rejection authority");
    let project = temporary.path().join("project");
    let created = compact_success(&["new", path(&project), "--name", "project"]);
    let initial = compact_field(compact_record(&created, "revision"), "id").unwrap();
    let creation = format!("request base={initial}\ncreate.module as=$module name=before\n");
    let planned = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input",
        &creation,
    ]);
    let plan = compact_field(compact_record(&planned, "plan"), "token").unwrap();
    let applied = compact_success(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "--input",
        &creation,
        "--plan",
        plan,
    ]);
    let owner = compact_field(
        applied
            .iter()
            .find(|record| record.operation == "identity")
            .unwrap(),
        "id",
    )
    .unwrap()
    .to_owned();
    let revision = compact_field(compact_record(&applied, "revision"), "result")
        .unwrap()
        .to_owned();
    let project_text = path(&project);
    let common = ["--project", project_text, "change", "plan", "rename.owner"];

    assert_direct_rename_rejection(
        &project,
        &[common.as_slice(), &["--owner", &owner, "--name", "renamed"]].concat(),
        "cli_usage",
        2,
        &revision,
    );
    assert_direct_rename_rejection(
        &project,
        &[
            common.as_slice(),
            &["--base", &revision, "--name", "renamed"],
        ]
        .concat(),
        "cli_usage",
        2,
        &revision,
    );
    assert_direct_rename_rejection(
        &project,
        &[common.as_slice(), &["--base", &revision, "--owner", &owner]].concat(),
        "cli_usage",
        2,
        &revision,
    );
    assert_direct_rename_rejection(
        &project,
        &[
            common.as_slice(),
            &[
                "--base", &revision, "--base", &revision, "--owner", &owner, "--name", "renamed",
            ],
        ]
        .concat(),
        "cli_usage",
        2,
        &revision,
    );
    for arguments in [
        vec![
            "--base",
            revision.as_str(),
            "--owner",
            owner.as_str(),
            "--name",
            "renamed",
            "--unknown",
            "value",
        ],
        vec![
            "--base",
            revision.as_str(),
            "--owner",
            owner.as_str(),
            "--name",
            "renamed",
            "extra",
        ],
    ] {
        assert_direct_rename_rejection(
            &project,
            &[common.as_slice(), arguments.as_slice()].concat(),
            "cli_usage",
            2,
            &revision,
        );
    }
    for (arguments, code) in [
        (
            vec![
                "--base",
                "rev_bad",
                "--owner",
                owner.as_str(),
                "--name",
                "renamed",
            ],
            "revision_identity_length",
        ),
        (
            vec![
                "--base",
                revision.as_str(),
                "--owner",
                "mod_bad",
                "--name",
                "renamed",
            ],
            "semantic_identity_length",
        ),
        (
            vec![
                "--base",
                revision.as_str(),
                "--owner",
                revision.as_str(),
                "--name",
                "renamed",
            ],
            "kernel_owner_identity_domain",
        ),
        (
            vec![
                "--base",
                revision.as_str(),
                "--owner",
                owner.as_str(),
                "--name",
                "9invalid",
            ],
            "kernel_name",
        ),
    ] {
        assert_direct_rename_rejection(
            &project,
            &[common.as_slice(), arguments.as_slice()].concat(),
            code,
            2,
            &revision,
        );
    }
    let oversized_idempotency = "x".repeat(129);
    let oversized_intent = "x".repeat(4097);
    for (option, value, code) in [
        ("--idempotency", "", "change_idempotency"),
        (
            "--idempotency",
            oversized_idempotency.as_str(),
            "change_idempotency",
        ),
        ("--intent", oversized_intent.as_str(), "change_intent_bytes"),
    ] {
        let arguments = [
            common.as_slice(),
            &[
                "--base", &revision, "--owner", &owner, "--name", "renamed", option, value,
            ],
        ]
        .concat();
        assert_direct_rename_rejection(&project, &arguments, code, 2, &revision);
    }

    let apply = [
        "--project",
        project_text,
        "change",
        "apply",
        "rename.owner",
        "--base",
        &revision,
        "--owner",
        &owner,
        "--name",
        "renamed",
    ];
    assert_direct_rename_rejection(&project, &apply, "cli_usage", 2, &revision);
    let malformed_plan = [apply.as_slice(), &["--plan", "plan_bad"]].concat();
    assert_direct_rename_rejection(
        &project,
        &malformed_plan,
        "change_plan_length",
        2,
        &revision,
    );
    let predecessor_plan = "plan_0000000000000000000000000000000000000000000000000000000000000000";
    let predecessor = [apply.as_slice(), &["--plan", predecessor_plan]].concat();
    assert_direct_rename_rejection(&project, &predecessor, "change_plan_length", 2, &revision);
    let wrong_plan = "plan_00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    let mismatched = [apply.as_slice(), &["--plan", wrong_plan]].concat();
    assert_direct_rename_rejection(
        &project,
        &mismatched,
        "change_request_commitment_mismatch",
        2,
        &revision,
    );

    let nonexistent = temporary.path().join("does-not-exist");
    let rejected = compact_failure_output(command(&[
        "--project",
        path(&nonexistent),
        "change",
        "apply",
        "rename.owner",
        "--base",
        &revision,
        "--owner",
        &owner,
        "--name",
        "renamed",
        "--plan",
        wrong_plan,
    ]));
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some("change_request_commitment_mismatch")
    );
    assert!(!nonexistent.exists());
}

#[test]
fn direct_rename_stale_and_absent_exact_owners_leave_head_unchanged() {
    let temporary = tempfile::TempDir::new().expect("temporary stale rename authority");
    let project = temporary.path().join("project");
    let created = compact_success(&["new", path(&project), "--name", "project"]);
    let initial = compact_field(compact_record(&created, "revision"), "id").unwrap();
    let creation = format!("request base={initial}\ncreate.module as=$module name=before\n");
    let planned = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input",
        &creation,
    ]);
    let creation_plan = compact_field(compact_record(&planned, "plan"), "token").unwrap();
    let applied = compact_success(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "--input",
        &creation,
        "--plan",
        creation_plan,
    ]);
    let owner = compact_field(
        applied
            .iter()
            .find(|record| record.operation == "identity")
            .unwrap(),
        "id",
    )
    .unwrap()
    .to_owned();
    let base = compact_field(compact_record(&applied, "revision"), "result")
        .unwrap()
        .to_owned();

    let first = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "rename.owner",
        "--base",
        &base,
        "--owner",
        &owner,
        "--name",
        "first",
    ]);
    let first_plan = compact_field(compact_record(&first, "plan"), "token").unwrap();
    let second = compact_success(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "rename.owner",
        "--base",
        &base,
        "--owner",
        &owner,
        "--name",
        "second",
    ]);
    let second_plan = compact_field(compact_record(&second, "plan"), "token")
        .unwrap()
        .to_owned();
    let accepted = compact_success(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "rename.owner",
        "--base",
        &base,
        "--owner",
        &owner,
        "--name",
        "first",
        "--plan",
        first_plan,
    ]);
    let accepted_revision = compact_field(compact_record(&accepted, "revision"), "result")
        .unwrap()
        .to_owned();
    assert_direct_rename_rejection(
        &project,
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "rename.owner",
            "--base",
            &base,
            "--owner",
            &owner,
            "--name",
            "second",
            "--plan",
            &second_plan,
        ],
        "change_authored_stale_base",
        7,
        &accepted_revision,
    );

    let absent = if owner == "mod_00000000000000000000000000000001" {
        "mod_00000000000000000000000000000002"
    } else {
        "mod_00000000000000000000000000000001"
    };
    assert_direct_rename_rejection(
        &project,
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "rename.owner",
            "--base",
            &accepted_revision,
            "--owner",
            absent,
            "--name",
            "missing",
        ],
        "change_authored_owner_missing",
        2,
        &accepted_revision,
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
    let plan = compact_field(compact_record(&planned, "plan"), "token").unwrap();
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
        "draft",
        "history",
        "review",
        "backup",
        "restore",
        "doctor",
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
        let result = compact_failure_output(command(&[removed]));
        assert_eq!(
            compact_field(compact_record(&result, "diagnostic"), "code"),
            Some("cli_usage"),
            "{removed}"
        );
    }
    let removed_package_stage = compact_failure_output(command(&["package", "stage", "old.lkja"]));
    assert_eq!(
        compact_field(compact_record(&removed_package_stage, "diagnostic"), "code"),
        Some("cli_usage")
    );

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
    let value = compact_failure_output(command(&["--project", path(temporary.path()), "status"]));
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
    assert!(compact_field(compact_record(&status, "schema"), "capabilities").is_some());
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
    let second_receipt = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&second), "--name", "second_project"],
    ));
    assert_eq!(
        compact_field(compact_record(&second_receipt, "diagnostic"), "code"),
        Some("new_destination_not_empty")
    );
    assert!(
        std::fs::read_dir(&second)
            .expect("preserved empty destination")
            .next()
            .is_none()
    );

    let repeated = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&first)],
    ));
    assert_eq!(
        compact_field(compact_record(&repeated, "diagnostic"), "code"),
        Some("new_destination_not_empty")
    );

    for alias in ["web", "server", "service"] {
        let destination = temporary.path().join(format!("rejected-{alias}"));
        let output = compact_failure_output(command_at(
            &copied_binary,
            temporary.path(),
            &[
                "new",
                path(&destination),
                "--template",
                alias,
                "--name",
                "rejected",
            ],
        ));
        assert_eq!(
            compact_field(compact_record(&output, "diagnostic"), "code"),
            Some("cli_usage"),
            "{alias}"
        );
        assert!(!destination.exists(), "{alias}");
    }

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

    let non_directory_parent = temporary.path().join("non-directory-parent");
    std::fs::write(&non_directory_parent, b"preserve\n").expect("non-directory parent");
    let child = non_directory_parent.join("child");
    let rejected = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&child)],
    ));
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        Some("new_destination_parent_type")
    );
    assert_eq!(
        std::fs::read(&non_directory_parent).expect("preserved parent file"),
        b"preserve\n"
    );

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
fn copied_release_owned_closure_deletion_is_reviewed_atomic_and_reopenable() {
    let temporary = tempfile::TempDir::new().expect("isolated owned-closure workspace");
    let copied_binary = temporary.path().join("lkjscript");
    copy_executable(&binary(), &copied_binary);
    let project = temporary.path().join("project");
    let created = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&project), "--name", "closure-workflow"],
    );
    let initial = compact_field(compact_record(&created, "revision"), "id")
        .expect("initial revision")
        .to_owned();
    let creation = format!(
        "request base={initial} idempotency=closure-fixture-create\n\
         create.module as=$alpha name=alpha\n\
         create.module as=$beta name=beta\n\
         create.record as=$payload module=$alpha name=Payload visibility=public\n\
         add.field as=$first record=$payload name=first type=unit\n\
         add.field as=$second record=$payload name=second type=unit\n\
         type.named as=@payload-type declaration=$payload\n\
         expression.local as=$body value=$parameter\n\
         create.function as=$consumer module=$beta name=consumer visibility=public result=@payload-type effect=pure body=$body\n\
         add.parameter as=$parameter function=$consumer name=payload type=@payload-type\n"
    );
    let creation_path = temporary.path().join("create.lkjc");
    std::fs::write(&creation_path, creation).expect("write owned-closure fixture request");
    let creation_plan = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "--input-file",
            path(&creation_path),
        ],
    );
    let creation_token = compact_field(compact_record(&creation_plan, "plan"), "token")
        .expect("fixture plan token")
        .to_owned();
    let identities = creation_plan
        .iter()
        .filter(|record| record.operation == "identity")
        .map(|record| {
            (
                compact_field(record, "symbol")
                    .expect("identity symbol")
                    .to_owned(),
                compact_field(record, "id")
                    .expect("identity value")
                    .to_owned(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(identities.len(), 8);
    let created_graph = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "--input-file",
            path(&creation_path),
            "--plan",
            &creation_token,
        ],
    );
    let fixture_revision = compact_field(compact_record(&created_graph, "revision"), "result")
        .expect("fixture revision")
        .to_owned();
    let alpha = &identities["$alpha"];
    let beta = &identities["$beta"];
    let payload = &identities["$payload"];
    let consumer = &identities["$consumer"];

    let unchanged = content_inventory(&project);
    for (name, body, code) in [
        (
            "reject-non-leaf",
            format!(
                "request base={fixture_revision}\n\
                 delete.owner owner={payload} policy=reject\n"
            ),
            "change_delete_owned_children",
        ),
        (
            "reject-live-reference",
            format!(
                "request base={fixture_revision}\n\
                 delete.owner owner={payload} policy=owned-closure\n"
            ),
            "change_delete_live_reference",
        ),
        (
            "reject-cascade-field",
            format!(
                "request base={fixture_revision}\n\
                 delete.owner owner={payload} policy=reject cascade=true\n"
            ),
            "change_field_unknown",
        ),
        (
            "reject-cascade-policy",
            format!(
                "request base={fixture_revision}\n\
                 delete.owner owner={payload} policy=cascade\n"
            ),
            "change_delete_policy",
        ),
        (
            "reject-recursive-policy",
            format!(
                "request base={fixture_revision}\n\
                 delete.owner owner={payload} policy=recursive\n"
            ),
            "change_delete_policy",
        ),
        (
            "reject-deep-policy",
            format!(
                "request base={fixture_revision}\n\
                 delete.owner owner={payload} policy=deep\n"
            ),
            "change_delete_policy",
        ),
        (
            "reject-missing-policy",
            format!(
                "request base={fixture_revision}\n\
                 delete.owner owner={payload}\n"
            ),
            "change_field_missing",
        ),
        (
            "reject-duplicate-policy",
            format!(
                "request base={fixture_revision}\n\
                 delete.owner owner={payload} policy=reject policy=owned-closure\n"
            ),
            "control_duplicate_field",
        ),
        (
            "reject-foreign-owner-form",
            format!(
                "request base={fixture_revision}\n\
                 delete.owner owner=pkg_00000000000000000000000000000001/{payload} policy=owned-closure\n"
            ),
            "change_field_value",
        ),
    ] {
        let rejected_path = temporary.path().join(format!("{name}.lkjc"));
        std::fs::write(&rejected_path, body).expect("write rejected closure request");
        let rejected = compact_failure_output(command_at(
            &copied_binary,
            temporary.path(),
            &[
                "--project",
                path(&project),
                "change",
                "plan",
                "--input-file",
                path(&rejected_path),
            ],
        ));
        assert_eq!(
            compact_field(compact_record(&rejected, "diagnostic"), "code"),
            Some(code),
            "{name}"
        );
        assert_eq!(content_inventory(&project), unchanged, "{name}");
        assert_eq!(
            current_revision_at(&copied_binary, temporary.path(), &project),
            fixture_revision,
            "{name}"
        );
    }

    let stale_request = format!(
        "request base={fixture_revision} idempotency=owned-closure-public-1 intent=reviewed-closure\n\
         delete.owner owner={payload} policy=owned-closure\n\
         delete.owner owner={consumer} policy=owned-closure\n"
    );
    let stale_request_path = temporary.path().join("stale-delete.lkjc");
    std::fs::write(&stale_request_path, stale_request).expect("write stale deletion request");
    let stale_plan = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "--input-file",
            path(&stale_request_path),
        ],
    );
    let stale_token = compact_field(compact_record(&stale_plan, "plan"), "token")
        .expect("stale plan token")
        .to_owned();
    let rename_plan = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "rename.owner",
            "--base",
            &fixture_revision,
            "--owner",
            beta,
            "--name",
            "beta_live",
        ],
    );
    let rename_token = compact_field(compact_record(&rename_plan, "plan"), "token")
        .expect("base-advance plan")
        .to_owned();
    let renamed = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "rename.owner",
            "--base",
            &fixture_revision,
            "--owner",
            beta,
            "--name",
            "beta_live",
            "--plan",
            &rename_token,
        ],
    );
    let deletion_base = compact_field(compact_record(&renamed, "revision"), "result")
        .expect("advanced deletion base")
        .to_owned();
    let before_stale_apply = content_inventory(&project);
    let stale = compact_failure_output_with_status(
        command_at(
            &copied_binary,
            temporary.path(),
            &[
                "--project",
                path(&project),
                "change",
                "apply",
                "--input-file",
                path(&stale_request_path),
                "--plan",
                &stale_token,
            ],
        ),
        7,
    );
    assert_eq!(
        compact_field(compact_record(&stale, "diagnostic"), "code"),
        Some("change_authored_stale_base")
    );
    assert_eq!(content_inventory(&project), before_stale_apply);
    assert_eq!(
        current_revision_at(&copied_binary, temporary.path(), &project),
        deletion_base
    );

    let deletion = format!(
        "request base={deletion_base} idempotency=owned-closure-public-1 intent=reviewed-closure\n\
         delete.owner owner={payload} policy=owned-closure\n\
         delete.owner owner={consumer} policy=owned-closure\n"
    );
    let deletion_path = temporary.path().join("delete.lkjc");
    std::fs::write(&deletion_path, deletion).expect("write reviewed closure deletion");
    let logical_plan = temporary.path().join("delete.logical-plan");
    let before_plan = content_inventory(&project);
    let planned = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "--input-file",
            path(&deletion_path),
            "--output",
            path(&logical_plan),
        ],
    );
    let plan = compact_field(compact_record(&planned, "plan"), "token")
        .expect("reviewed closure plan")
        .to_owned();
    assert_eq!(plan.len(), 5 + 128);
    assert_eq!(content_inventory(&project), before_plan);
    assert_eq!(
        compact_field(compact_record(&planned, "summary"), "deleted"),
        Some("6")
    );
    assert_eq!(
        compact_field(compact_record(&planned, "summary"), "retirements"),
        Some("6")
    );
    let repeated_plan = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "--input-file",
            path(&deletion_path),
        ],
    );
    assert_eq!(
        compact_field(compact_record(&repeated_plan, "plan"), "token"),
        Some(plan.as_str())
    );
    let decoded = decode_logical_change_plan(BufReader::new(
        File::open(&logical_plan).expect("open owned-closure logical plan"),
    ))
    .expect("strictly decode owned-closure logical plan");
    assert_eq!(decoded.token.to_string(), plan);
    let plan_records = parse_records(
        "owned-closure logical plan",
        &std::fs::read(&logical_plan).expect("read owned-closure logical plan"),
    )
    .expect("parse owned-closure logical plan records");
    let expected_deleted = [
        "$payload",
        "$first",
        "$second",
        "$consumer",
        "$parameter",
        "$body",
    ]
    .into_iter()
    .map(|symbol| identities[symbol].as_str())
    .collect::<BTreeSet<_>>();
    let planned_deleted = plan_records
        .iter()
        .filter(|record| {
            record.operation == "logical-plan.owner"
                && compact_field(record, "class-deleted") == Some("true")
        })
        .filter_map(|record| compact_field(record, "owner"))
        .collect::<BTreeSet<_>>();
    assert_eq!(planned_deleted, expected_deleted);
    let planned_retirements = plan_records
        .iter()
        .filter(|record| {
            record.operation == "logical-plan.retirement"
                && compact_field(record, "before-present") == Some("false")
                && compact_field(record, "after-present") == Some("true")
        })
        .filter_map(|record| compact_field(record, "owner"))
        .collect::<BTreeSet<_>>();
    assert_eq!(planned_retirements, expected_deleted);

    let changed_intent = format!(
        "request base={deletion_base} idempotency=owned-closure-public-1 intent=unreviewed-change\n\
         delete.owner owner={payload} policy=owned-closure\n\
         delete.owner owner={consumer} policy=owned-closure\n"
    );
    let mismatch = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "--input",
            &changed_intent,
            "--plan",
            &plan,
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&mismatch, "diagnostic"), "code"),
        Some("change_request_commitment_mismatch")
    );
    assert_eq!(content_inventory(&project), before_plan);

    let applied = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "--input-file",
            path(&deletion_path),
            "--plan",
            &plan,
        ],
    );
    assert_eq!(compact_field(&applied[0], "status"), Some("accepted"));
    let accepted = compact_field(compact_record(&applied, "revision"), "result")
        .expect("owned-closure accepted revision")
        .to_owned();

    for (kind, owner) in [("record", payload), ("pure_function", consumer)] {
        let absent = compact_failure_output(command_at(
            &copied_binary,
            temporary.path(),
            &["--project", path(&project), "inspect", "owner", kind, owner],
        ));
        assert_eq!(
            compact_field(compact_record(&absent, "diagnostic"), "code"),
            Some("owner_not_found")
        );
    }
    let status = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["--project", path(&project), "status"],
    );
    assert_eq!(
        compact_field(compact_record(&status, "revision"), "id"),
        Some(accepted.as_str())
    );
    for (owner, name) in [(alpha, "alpha"), (beta, "beta_live")] {
        let inspected = compact_success_at(
            &copied_binary,
            temporary.path(),
            &[
                "--project",
                path(&project),
                "inspect",
                "owner",
                "module",
                owner,
            ],
        );
        assert_eq!(
            compact_field(compact_record(&inspected, "revision"), "observed"),
            Some(accepted.as_str())
        );
        assert_eq!(
            compact_field(compact_record(&inspected, "owner"), "name"),
            Some(name)
        );
        let relations = compact_success_at(
            &copied_binary,
            temporary.path(),
            &[
                "--project",
                path(&project),
                "query",
                "relations",
                owner,
                "--direction",
                "incoming",
                "--kind",
                "declaration_module",
            ],
        );
        assert_eq!(
            compact_field(compact_record(&relations, "summary"), "returned"),
            Some("0")
        );
    }
    let owners = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "owners",
            "--limit",
            "16",
        ],
    );
    assert_eq!(
        owners
            .iter()
            .filter(|record| record.operation == "owner")
            .filter_map(|record| compact_field(record, "id"))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([alpha.as_str(), beta.as_str()])
    );
    for (name, parent) in [("Payload", alpha.as_str()), ("consumer", beta.as_str())] {
        let found = compact_success_at(
            &copied_binary,
            temporary.path(),
            &[
                "--project",
                path(&project),
                "query",
                "find",
                "declaration",
                name,
                "--parent",
                parent,
            ],
        );
        assert_eq!(
            compact_field(compact_record(&found, "summary"), "match"),
            Some("false")
        );
    }
    let deleted_parent = compact_failure_output(command_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "query",
            "find",
            "field",
            "first",
            "--parent",
            payload,
        ],
    ));
    assert_eq!(
        compact_field(compact_record(&deleted_parent, "diagnostic"), "code"),
        Some("query_parent_not_found")
    );

    let before_replay = content_inventory(&project);
    let replay = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "--input-file",
            path(&deletion_path),
            "--plan",
            &plan,
        ],
    );
    assert_eq!(
        compact_field(compact_record(&replay, "result"), "status"),
        Some("already-accepted")
    );
    assert_eq!(
        compact_field(compact_record(&replay, "revision"), "result"),
        Some(accepted.as_str())
    );
    assert_eq!(content_inventory(&project), before_replay);
    assert_eq!(
        current_revision_at(&copied_binary, temporary.path(), &project),
        accepted
    );
    assert!(!temporary.path().join("Cargo.toml").exists());
    assert!(!project.join(".lkjscript").exists());
}

#[test]
fn copied_binary_direct_rename_completes_a_normalized_external_workflow() {
    let temporary = tempfile::TempDir::new().expect("isolated direct rename workspace");
    let copied_binary = temporary.path().join("lkjscript");
    copy_executable(&binary(), &copied_binary);
    let project = temporary.path().join("project");
    let created = compact_success_at(
        &copied_binary,
        temporary.path(),
        &["new", path(&project), "--name", "project"],
    );
    let initial = compact_field(compact_record(&created, "revision"), "id").unwrap();
    let creation = format!("request base={initial}\ncreate.module as=$module name=before\n");
    let planned = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "--input",
            &creation,
        ],
    );
    let creation_plan = compact_field(compact_record(&planned, "plan"), "token").unwrap();
    let module = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "--input",
            &creation,
            "--plan",
            creation_plan,
        ],
    );
    let owner = compact_field(
        module
            .iter()
            .find(|record| record.operation == "identity")
            .unwrap(),
        "id",
    )
    .unwrap()
    .to_owned();
    let base = compact_field(compact_record(&module, "revision"), "result")
        .unwrap()
        .to_owned();
    let logical_plan = temporary.path().join("rename.logical-plan");
    let rename_plan = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "plan",
            "rename.owner",
            "--base",
            &base,
            "--owner",
            &owner,
            "--name",
            "after",
            "--output",
            path(&logical_plan),
        ],
    );
    let plan = compact_field(compact_record(&rename_plan, "plan"), "token").unwrap();
    let decoded = decode_logical_change_plan(BufReader::new(
        File::open(&logical_plan).expect("open copied-binary logical plan"),
    ))
    .expect("strictly decode copied-binary logical plan");
    assert_eq!(decoded.token, plan);
    assert_eq!(
        compact_field(compact_record(&rename_plan, "plan-output"), "status"),
        Some("published")
    );
    let renamed = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "change",
            "apply",
            "rename.owner",
            "--base",
            &base,
            "--owner",
            &owner,
            "--name",
            "after",
            "--plan",
            plan,
        ],
    );
    assert_eq!(compact_field(&renamed[0], "status"), Some("accepted"));
    let accepted = compact_field(compact_record(&renamed, "revision"), "result").unwrap();
    let inspected = compact_success_at(
        &copied_binary,
        temporary.path(),
        &[
            "--project",
            path(&project),
            "inspect",
            "owner",
            "module",
            &owner,
        ],
    );
    assert_eq!(
        compact_field(compact_record(&inspected, "revision"), "observed"),
        Some(accepted)
    );
    assert_eq!(
        compact_field(compact_record(&inspected, "owner"), "name"),
        Some("after")
    );
    assert!(!temporary.path().join("Cargo.toml").exists());
    assert!(!project.join(".lkjscript").exists());
}

#[test]
fn builtin_bytes_reproduce_maintained_authority() {
    let temporary = tempfile::TempDir::new().expect("temporary built-in export");
    let builtin = compact_success(&["package", "builtin", "inspect"]);
    let exported = temporary.path().join("standard.lkja");
    let export = compact_success(&[
        "package",
        "builtin",
        "export",
        "--kind",
        "artifact",
        "--output",
        path(&exported),
    ]);
    assert_eq!(
        compact_field(compact_record(&export, "package"), "id"),
        compact_field(compact_record(&builtin, "package"), "id")
    );
    assert_eq!(
        std::fs::read(exported).expect("exported built-in"),
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard/generated/standard.lkja")
        )
        .expect("maintained standard artifact")
    );
    let transport = temporary.path().join("standard.lkjp");
    compact_success(&[
        "package",
        "builtin",
        "export",
        "--kind",
        "transport",
        "--output",
        path(&transport),
    ]);
    assert_eq!(
        std::fs::read(transport).expect("exported transport"),
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard/generated/standard.lkjp")
        )
        .expect("maintained standard transport")
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
