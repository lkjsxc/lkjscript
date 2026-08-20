#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use lkjscript::project::{PROJECT_CHANGE_VERSION, ProjectChangeRequest};
use lkjscript::transaction::{NodeTarget, TransactionOp, TransactionResponseSpec};
use lkjscript::{DraftSymbol, Revision, WorkspaceId};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::Mutex;

static CHECKED_PROJECT_LOCK: Mutex<()> = Mutex::new(());

fn cli(current: &Path, arguments: &[String], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .current_dir(current)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn project CLI");
    child
        .stdin
        .take()
        .expect("project CLI stdin")
        .write_all(input)
        .expect("write project CLI input");
    child.wait_with_output().expect("project CLI output")
}

fn value(output: &Output, expected_exit: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "unexpected exit\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    serde_json::from_slice(&output.stdout).expect("strict project response JSON")
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn superseded_agent_and_command_local_build_paths_reject() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    for arguments in [
        strings(&["agent", "orient"]),
        strings(&["release", "build"]),
        strings(&["app", "build"]),
    ] {
        let output = cli(repository, &arguments, &[]);
        assert_eq!(output.status.code(), Some(2));
        assert!(
            serde_json::from_slice::<Value>(&output.stdout).is_ok(),
            "rejected predecessor must retain strict machine framing: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
fn superseded_project_change_document_and_session_versions_reject() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let initialized = value(
        &cli(temporary.path(), &strings(&["init", "project"]), &[]),
        0,
    );
    let project = temporary.path().join("project");
    let workspace = initialized["result"]["data"]["workspace"]
        .as_str()
        .expect("workspace");
    let orientation = value(&cli(&project, &strings(&["orient"]), &[]), 0);
    let schema = orientation["result"]["data"]["data"]["machine_schema"]
        .as_str()
        .expect("machine schema");

    let old_change = serde_json::json!({
        "version": 1,
        "workspace": workspace,
        "base_revision": 0,
        "operations": [],
        "response": {"return_symbols": []}
    });
    let rejected_change = value(
        &cli(
            &project,
            &strings(&["change", "validate"]),
            old_change.to_string().as_bytes(),
        ),
        2,
    );
    assert_eq!(
        rejected_change["result"]["data"]["code"],
        "protocol_version"
    );

    let old_document = format!(
        "document {{ version 1 schema {schema} workspace {workspace} base_revision 0 scope (workspace) edits [] return_symbols [] }}"
    );
    let rejected_document = value(
        &cli(
            &project,
            &strings(&["change", "validate", "--document"]),
            old_document.as_bytes(),
        ),
        2,
    );
    assert_eq!(
        rejected_document["result"]["data"]["code"],
        "protocol_version"
    );

    let session_input = concat!(
        "{\"version\":1,\"request_id\":1,\"request\":{\"kind\":\"status\"}}\n",
        "{\"version\":2,\"request_id\":2,\"request\":{\"kind\":\"shutdown\"}}\n"
    );
    let session = cli(&project, &strings(&["session"]), session_input.as_bytes());
    assert_eq!(session.status.code(), Some(0));
    let responses = session
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<Value>(line).expect("session response"))
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 2);
    assert!(responses[0].get("request_id").is_none());
    assert_eq!(responses[0]["version"], 2);
    assert_eq!(responses[0]["result"]["data"]["code"], "protocol_version");
    assert_eq!(responses[1]["request_id"], 2);
}

#[test]
fn public_project_workflow_discovers_validates_applies_reviews_and_recovers() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let project_root = temporary.path().join("application");
    let initialized = value(
        &cli(temporary.path(), &strings(&["init", "application"]), &[]),
        0,
    );
    assert_eq!(initialized["version"], 2);
    assert_eq!(initialized["result"]["kind"], "initialized");
    assert_eq!(initialized["result"]["data"]["revision"], 0);
    let workspace = initialized["result"]["data"]["workspace"]
        .as_str()
        .expect("workspace")
        .parse::<WorkspaceId>()
        .expect("workspace identity");

    let descendant = project_root.join("nested/directory");
    fs::create_dir_all(&descendant).expect("descendant");
    let initial_status = value(&cli(&descendant, &strings(&["status"]), &[]), 0);
    assert_eq!(
        initial_status["result"]["data"]["workspace"],
        workspace.to_string()
    );
    assert_eq!(initial_status["result"]["data"]["revision"], 0);

    let orientation = value(&cli(&descendant, &strings(&["orient"]), &[]), 0);
    let orientation_digest = orientation["result"]["data"]["data"]["orientation_digest"]
        .as_str()
        .expect("orientation digest");
    let unchanged = value(
        &cli(
            &descendant,
            &strings(&["orient", "--known-digest", orientation_digest]),
            &[],
        ),
        0,
    );
    assert_eq!(unchanged["result"]["data"]["kind"], "unchanged");

    let request = ProjectChangeRequest {
        version: PROJECT_CHANGE_VERSION,
        workspace,
        base_revision: Revision::INITIAL,
        idempotency_key: None,
        operations: vec![TransactionOp::CreatePackage {
            symbol: DraftSymbol::new("package"),
            name: "demo".into(),
        }],
        response: TransactionResponseSpec {
            return_symbols: vec![DraftSymbol::new("package")],
        },
    };
    let request_bytes = serde_json::to_vec(&request).expect("change JSON");
    let validated = value(
        &cli(
            &descendant,
            &strings(&["change", "validate"]),
            &request_bytes,
        ),
        0,
    );
    assert_eq!(
        validated["result"]["data"]["transaction"]["published"],
        false
    );
    assert!(validated["result"]["data"].get("continuation").is_none());
    assert_eq!(
        value(&cli(&descendant, &strings(&["status"]), &[]), 0)["result"]["data"]["revision"],
        0
    );

    let applied = value(
        &cli(&descendant, &strings(&["change", "apply"]), &request_bytes),
        0,
    );
    assert_eq!(applied["result"]["data"]["transaction"]["published"], true);
    assert_eq!(applied["result"]["data"]["transaction"]["revision"], 1);
    assert_eq!(applied["result"]["data"]["continuation"]["version"], 1);
    assert_eq!(applied["result"]["data"]["continuation"]["revision"], 1);
    assert_eq!(
        applied["result"]["data"]["continuation"]["session_local_aliases_invalidated"],
        true
    );
    let package = applied["result"]["data"]["transaction"]["returned_bindings"][0][1]
        .as_str()
        .expect("created package")
        .to_owned();

    let context = cli(
        &descendant,
        &strings(&["context", "--purpose", "refactor", "--target", &package]),
        &[],
    );
    let context_value = value(&context, 0);
    assert_eq!(context_value["result"]["data"]["kind"], "changed");
    let packet = &context_value["result"]["data"]["data"];
    let packet_digest = packet["digest"].as_str().expect("packet digest");
    let schema = packet["payload"]["schema_digest"]
        .as_str()
        .expect("schema digest");
    let alias = packet["payload"]["aliases"]
        .as_array()
        .expect("aliases")
        .iter()
        .find(|entry| entry["node"] == package)
        .and_then(|entry| entry["alias"].as_str())
        .expect("package alias");
    let context_path = temporary.path().join("context.json");
    fs::write(&context_path, &context.stdout).expect("save exact context response");

    let document = format!(
        "document {{ version 2 schema \"{schema}\" packet \"{packet_digest}\" workspace \"{workspace}\" base_revision 1 scope (workspace) edits [ (rename_node {{ node (existing @{alias}) name renamed }}) ] return_symbols [] }}"
    );
    let context_path = context_path.to_str().expect("UTF-8 context path");
    let validate_document = value(
        &cli(
            &descendant,
            &strings(&[
                "change",
                "validate",
                "--document",
                "--context",
                context_path,
            ]),
            document.as_bytes(),
        ),
        0,
    );
    assert_eq!(
        validate_document["result"]["data"]["transaction"]["revision"],
        2
    );
    assert_eq!(
        validate_document["result"]["data"]["transaction"]["published"],
        false
    );
    let apply_document = value(
        &cli(
            &descendant,
            &strings(&["change", "apply", "--document", "--context", context_path]),
            document.as_bytes(),
        ),
        0,
    );
    assert_eq!(
        apply_document["result"]["data"]["transaction"]["revision"],
        2
    );

    let stale = value(
        &cli(
            &descendant,
            &strings(&["change", "apply", "--document", "--context", context_path]),
            document.as_bytes(),
        ),
        2,
    );
    assert_eq!(stale["result"]["data"]["code"], "revision_conflict");

    let log = value(
        &cli(&descendant, &strings(&["log", "--limit", "2"]), &[]),
        0,
    );
    assert_eq!(
        log["result"]["data"]["records"]
            .as_array()
            .expect("history records")
            .len(),
        2
    );
    assert_eq!(log["result"]["data"]["records"][0]["revision"], 2);
    assert!(log["result"]["data"]["records"][0].get("created").is_none());

    let shown = value(&cli(&descendant, &strings(&["show", "2"]), &[]), 0);
    assert_eq!(shown["result"]["data"]["record"]["revision"], 2);
    let diff = value(
        &cli(
            &descendant,
            &strings(&["diff", "--from", "1", "--to", "2"]),
            &[],
        ),
        0,
    );
    assert_eq!(diff["result"]["data"]["total"], 1);

    let inspected = value(&cli(&descendant, &strings(&["inspect", &package]), &[]), 0);
    assert_eq!(inspected["result"]["data"]["resolved"], package);
    assert_eq!(
        inspected["result"]["data"]["view"]["record"]["data"]["name"],
        "renamed"
    );

    let backup_path = temporary.path().join("backup");
    let backup = value(
        &cli(
            &descendant,
            &strings(&["backup", backup_path.to_str().expect("UTF-8 backup path")]),
            &[],
        ),
        0,
    );
    assert_eq!(backup["result"]["data"]["revision"], 2);
    let copied = value(&cli(&backup_path, &strings(&["doctor", "--deep"]), &[]), 0);
    assert_eq!(copied["result"]["data"]["workspace"], workspace.to_string());
    assert_eq!(copied["result"]["data"]["revision"], 2);
}

#[test]
fn semantic_query_and_function_proposal_are_bounded_and_base_exact() {
    let _checked_project = CHECKED_PROJECT_LOCK.lock().expect("checked project lock");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project = repository.join("applications/lkjwork");
    let project_path = project.to_str().expect("UTF-8 project path");

    let first = value(
        &cli(
            repository,
            &strings(&[
                "query",
                "function",
                "--root",
                "why_task",
                "--limit",
                "1",
                "--project",
                project_path,
            ]),
            &[],
        ),
        0,
    );
    assert_eq!(first["result"]["kind"], "query");
    assert_eq!(first["result"]["data"]["kind"], "changed");
    assert_eq!(
        first["result"]["data"]["data"]["items"]
            .as_array()
            .expect("query items")
            .len(),
        1
    );
    assert_eq!(
        first["result"]["data"]["data"]["omissions"]["truncated"],
        true
    );
    let continuation = first["result"]["data"]["data"]["continuation"]
        .as_str()
        .expect("query continuation");
    let digest = first["result"]["data"]["data"]["result_digest"]
        .as_str()
        .expect("query result digest");
    let second = value(
        &cli(
            repository,
            &strings(&[
                "query",
                "function",
                "--root",
                "why_task",
                "--limit",
                "1",
                "--continuation",
                continuation,
                "--project",
                project_path,
            ]),
            &[],
        ),
        0,
    );
    assert_ne!(
        first["result"]["data"]["data"]["items"][0],
        second["result"]["data"]["data"]["items"][0]
    );
    let unchanged = value(
        &cli(
            repository,
            &strings(&[
                "query",
                "function",
                "--root",
                "why_task",
                "--limit",
                "1",
                "--known-digest",
                digest,
                "--project",
                project_path,
            ]),
            &[],
        ),
        0,
    );
    assert_eq!(unchanged["result"]["data"]["kind"], "unchanged");
    let mismatched = value(
        &cli(
            repository,
            &strings(&[
                "query",
                "summary",
                "--root",
                "why_task",
                "--limit",
                "1",
                "--continuation",
                continuation,
                "--project",
                project_path,
            ]),
            &[],
        ),
        2,
    );
    assert_eq!(mismatched["result"]["data"]["code"], "invalid_cursor");

    let proposal = value(
        &cli(
            repository,
            &strings(&["proposal", "why_task", "--project", project_path]),
            &[],
        ),
        0,
    );
    assert_eq!(proposal["result"]["kind"], "proposal");
    let base_revision = proposal["result"]["data"]["revision"]
        .as_u64()
        .expect("proposal revision");
    let document = proposal["result"]["data"]["document"]
        .as_str()
        .expect("proposal document");
    assert!(document.contains("packet null"));
    let no_change = value(
        &cli(
            repository,
            &strings(&[
                "change",
                "validate",
                "--document",
                "--project",
                project_path,
            ]),
            document.as_bytes(),
        ),
        2,
    );
    assert_eq!(no_change["result"]["data"]["code"], "no_change");

    let owners = value(
        &cli(
            repository,
            &strings(&[
                "query",
                "owner_chain",
                "--root",
                "why_task",
                "--project",
                project_path,
            ]),
            &[],
        ),
        0,
    );
    let package = owners["result"]["data"]["data"]["items"]
        .as_array()
        .expect("owner items")
        .iter()
        .find(|item| item["data"]["kind"] == "package")
        .and_then(|item| item["data"]["node"].as_str())
        .expect("owning package")
        .parse()
        .expect("package identity");
    let workspace = proposal["result"]["data"]["workspace"]
        .as_str()
        .expect("proposal workspace")
        .parse::<WorkspaceId>()
        .expect("workspace identity");
    let temporary = tempfile::tempdir().expect("backup parent");
    let backup = temporary.path().join("backup");
    value(
        &cli(
            repository,
            &strings(&[
                "backup",
                backup.to_str().expect("UTF-8 backup path"),
                "--project",
                project_path,
            ]),
            &[],
        ),
        0,
    );
    let advance = ProjectChangeRequest {
        version: PROJECT_CHANGE_VERSION,
        workspace,
        base_revision: Revision::new(base_revision),
        idempotency_key: None,
        operations: vec![TransactionOp::RenameNode {
            node: NodeTarget::Existing(package),
            name: "lkjwork_stale_test".into(),
        }],
        response: TransactionResponseSpec::default(),
    };
    value(
        &cli(
            &backup,
            &strings(&["change", "apply"]),
            &serde_json::to_vec(&advance).expect("advance request"),
        ),
        0,
    );
    let stale = value(
        &cli(
            &backup,
            &strings(&["change", "validate", "--document"]),
            document.as_bytes(),
        ),
        2,
    );
    assert_eq!(stale["result"]["data"]["code"], "revision_conflict");
}

#[test]
fn checked_lkjwork_target_rebuild_is_public_and_byte_identical() {
    let _checked_project = CHECKED_PROJECT_LOCK.lock().expect("checked project lock");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project = repository.join("applications/lkjwork");
    let temporary = tempfile::tempdir().expect("target output root");
    let output_path = temporary.path().join("lkjwork-relative.lkja");
    let build = value(
        &cli(
            temporary.path(),
            &strings(&[
                "target",
                "build",
                "lkjwork",
                "--project",
                project.to_str().expect("UTF-8 project path"),
                "--output",
                "lkjwork-relative.lkja",
            ]),
            &[],
        ),
        0,
    );
    assert_eq!(build["result"]["data"]["published"], true);
    assert_eq!(build["result"]["data"]["revision"], 9);
    assert_eq!(
        fs::read(&output_path).expect("target-derived application"),
        fs::read(project.join("lkjwork.lkja")).expect("checked application")
    );

    let tested = value(
        &cli(
            repository,
            &strings(&[
                "target",
                "test",
                "lkjwork-application",
                "--project",
                project.to_str().expect("UTF-8 project path"),
            ]),
            &[],
        ),
        0,
    );
    assert_eq!(tested["result"]["data"]["published"], false);
    assert_eq!(tested["result"]["data"]["artifact"]["kind"], "application");
}

#[test]
fn project_session_is_correlated_restartable_and_rejects_stale_aliases() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let project_root = temporary.path().join("session-project");
    let initialized = value(
        &cli(
            temporary.path(),
            &strings(&["init", "session-project"]),
            &[],
        ),
        0,
    );
    let workspace = initialized["result"]["data"]["workspace"]
        .as_str()
        .expect("workspace")
        .parse::<WorkspaceId>()
        .expect("workspace identity");
    let request = ProjectChangeRequest {
        version: PROJECT_CHANGE_VERSION,
        workspace,
        base_revision: Revision::INITIAL,
        idempotency_key: None,
        operations: vec![TransactionOp::CreatePackage {
            symbol: DraftSymbol::new("package"),
            name: "session_package".into(),
        }],
        response: TransactionResponseSpec::default(),
    };
    let requests = [
        serde_json::json!({
            "version": 2,
            "request_id": 1,
            "request": {"kind": "context", "data": {"purpose": "orient"}}
        })
        .to_string(),
        serde_json::json!({
            "version": 2,
            "request_id": 2,
            "request": {"kind": "change_apply", "data": request}
        })
        .to_string(),
        serde_json::json!({
            "version": 2,
            "request_id": 3,
            "request": {"kind": "inspect", "data": {"selector": "@n1"}}
        })
        .to_string(),
        "{\"version\":2,".into(),
        serde_json::json!({
            "version": 2,
            "request_id": 2,
            "request": {"kind": "status"}
        })
        .to_string(),
        serde_json::json!({
            "version": 2,
            "request_id": 5,
            "request": {"kind": "status"}
        })
        .to_string(),
        serde_json::json!({
            "version": 2,
            "request_id": 6,
            "request": {"kind": "shutdown"}
        })
        .to_string(),
    ];
    let input = format!("{}\n", requests.join("\n"));
    let output = cli(
        temporary.path(),
        &strings(&[
            "session",
            "--project",
            project_root.to_str().expect("UTF-8 project path"),
        ]),
        input.as_bytes(),
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "session failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let responses = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<Value>(line).expect("session response JSON"))
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), requests.len());
    assert_eq!(responses[0]["request_id"], 1);
    assert_eq!(responses[0]["result"]["kind"], "context");
    assert_eq!(responses[1]["request_id"], 2);
    assert_eq!(responses[1]["result"]["data"]["transaction"]["revision"], 1);
    assert_eq!(responses[2]["request_id"], 3);
    assert_eq!(responses[2]["result"]["data"]["code"], "revision_conflict");
    assert!(responses[3].get("request_id").is_none());
    assert_eq!(responses[3]["result"]["data"]["code"], "protocol_malformed");
    assert_eq!(responses[4]["request_id"], 2);
    assert_eq!(responses[4]["result"]["data"]["code"], "protocol_malformed");
    assert_eq!(responses[5]["request_id"], 5);
    assert_eq!(responses[5]["result"]["data"]["revision"], 1);
    assert_eq!(responses[6]["request_id"], 6);
    assert_eq!(responses[6]["result"]["data"]["revision"], 1);

    let restarted = value(&cli(&project_root, &strings(&["status"]), &[]), 0);
    assert_eq!(restarted["result"]["data"]["revision"], 1);
}
