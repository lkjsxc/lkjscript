#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use lkjscript::daemon;
use lkjscript::error::ErrorCode;
use lkjscript::protocol::{Request, Response};
use lkjscript::schema::NodeKind;
use lkjscript::transaction::{TransactionMode, TransactionReceipt};
use lkjscript::workbench::{ContextPacket, decode_context_packet, parse_edit_plan};
use lkjscript::{Client, RequestId};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct RunningDaemon {
    child: Child,
    state: PathBuf,
}

impl RunningDaemon {
    fn start(state: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_lkjscriptd"))
            .args([
                "--state",
                state.to_str().expect("UTF-8 state path"),
                "--foreground",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn foreground daemon");
        let endpoint = daemon::endpoint_path(state);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !endpoint.exists() {
            if let Some(status) = child.try_wait().expect("daemon status") {
                panic!("daemon exited before readiness: {status}");
            }
            assert!(Instant::now() < deadline, "daemon readiness timeout");
            thread::sleep(Duration::from_millis(1));
        }
        Self {
            child,
            state: state.to_owned(),
        }
    }

    fn client(&self) -> Client {
        Client::new(daemon::endpoint_path(&self.state))
    }

    fn shutdown(mut self) {
        assert_eq!(
            self.client()
                .request(RequestId::new(900), &Request::Shutdown)
                .expect("shutdown response"),
            Response::Acknowledged
        );
        assert!(self.child.wait().expect("wait daemon").success());
    }
}

impl Drop for RunningDaemon {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn cli(arguments: &[String], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn CLI");
    child
        .stdin
        .take()
        .expect("CLI stdin")
        .write_all(input)
        .expect("write CLI input");
    child.wait_with_output().expect("CLI output")
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn packet_command(state: &Path, workspace: &str, revision: u64) -> Vec<String> {
    vec![
        "agent".into(),
        "context".into(),
        "--state".into(),
        state.display().to_string(),
        "--workspace".into(),
        workspace.into(),
        "--revision".into(),
        revision.to_string(),
        "--purpose".into(),
        "orient".into(),
    ]
}

fn transaction_receipt(output: &Output) -> TransactionReceipt {
    assert!(
        output.status.success(),
        "workbench action failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    match serde_json::from_slice::<Response>(&output.stdout).expect("logical response") {
        Response::TransactionReceipt(receipt) => receipt,
        response => panic!("unexpected response: {response:?}"),
    }
}

#[test]
fn public_workbench_packet_view_alias_plan_and_stale_rejection_are_exact() {
    let temporary = tempfile::tempdir().expect("state directory");
    let daemon = RunningDaemon::start(temporary.path());
    let create_arguments = vec![
        "agent".into(),
        "create".into(),
        "--state".into(),
        temporary.path().display().to_string(),
    ];
    let created = cli(&create_arguments, &[]);
    assert!(created.status.success());
    let Response::WorkspaceCreated(created) =
        serde_json::from_slice::<Response>(&created.stdout).expect("workspace response")
    else {
        panic!("workspace response");
    };
    let workspace = created.workspace.to_string();

    let context_arguments = packet_command(temporary.path(), &workspace, 0);
    let first_packet_output = cli(&context_arguments, &[]);
    let second_packet_output = cli(&context_arguments, &[]);
    assert!(first_packet_output.status.success());
    assert_eq!(first_packet_output.stdout, second_packet_output.stdout);
    let first_packet = decode_context_packet(&first_packet_output.stdout).expect("context packet");
    assert_eq!(first_packet.payload.nodes.len(), 1);
    assert_eq!(
        first_packet.payload.aliases[0].kind,
        NodeKind::WorkspaceRoot
    );

    let packet_zero_path = temporary.path().join("revision-zero.packet.json");
    fs::write(&packet_zero_path, &first_packet_output.stdout).expect("write packet");
    let view_arguments = vec![
        "agent".into(),
        "view".into(),
        "--packet".into(),
        packet_zero_path.display().to_string(),
        "--ids".into(),
    ];
    let first_view = cli(&view_arguments, &[]);
    let second_view = cli(&view_arguments, &[]);
    assert!(first_view.status.success());
    assert_eq!(first_view.stdout, second_view.stdout);
    let rendered = String::from_utf8(first_view.stdout).expect("UTF-8 view");
    assert!(rendered.contains("lkjscript semantic review v1"));
    assert!(rendered.contains("purpose orient"));
    assert!(rendered.contains("@n1="));
    assert!(!rendered.contains('\u{1b}'));

    let create_plan = format!(
        "plan {{ workspace {workspace} base_revision 0 operations [ \
         (create_package {{ symbol draft_1 name demo }}) \
         (create_module {{ symbol draft_2 package (draft draft_1) name focused }}) \
         (create_module {{ symbol draft_3 package (draft draft_1) name unrelated }}) \
         ] return_symbols [ draft_1 draft_2 draft_3 ] }}"
    );
    let validate_arguments = vec![
        "agent".into(),
        "validate".into(),
        "--state".into(),
        temporary.path().display().to_string(),
    ];
    let apply_arguments = vec![
        "agent".into(),
        "apply".into(),
        "--state".into(),
        temporary.path().display().to_string(),
    ];
    let predicted = transaction_receipt(&cli(&validate_arguments, create_plan.as_bytes()));
    assert!(!predicted.published);
    let committed = transaction_receipt(&cli(&apply_arguments, create_plan.as_bytes()));
    assert!(committed.published);
    let mut expected = predicted.clone();
    expected.published = true;
    assert_eq!(committed, expected);
    let bindings: std::collections::BTreeMap<_, _> = committed
        .returned_bindings
        .iter()
        .map(|(symbol, node)| (symbol.to_string(), *node))
        .collect();
    let focused_module = bindings["draft_2"];
    let unrelated_module = bindings["draft_3"];

    let focused_arguments = vec![
        "agent".into(),
        "context".into(),
        "--state".into(),
        temporary.path().display().to_string(),
        "--workspace".into(),
        workspace.clone(),
        "--revision".into(),
        "1".into(),
        "--purpose".into(),
        "refactor".into(),
        "--target".into(),
        focused_module.to_string(),
    ];
    let focused_output = cli(&focused_arguments, &[]);
    assert!(focused_output.status.success());
    let focused_packet =
        decode_context_packet(&focused_output.stdout).expect("focused context packet");
    let focused_nodes: std::collections::BTreeSet<_> = focused_packet
        .payload
        .nodes
        .iter()
        .map(|view| view.summary.node)
        .collect();
    assert!(focused_nodes.contains(&focused_module));
    assert!(
        !focused_nodes.contains(&unrelated_module),
        "owner traversal must not pull unrelated siblings into target context"
    );

    let context_one_output = cli(&packet_command(temporary.path(), &workspace, 1), &[]);
    assert!(context_one_output.status.success());
    let context_one: ContextPacket =
        decode_context_packet(&context_one_output.stdout).expect("revision-one packet");
    let package_alias = context_one
        .payload
        .aliases
        .iter()
        .find(|alias| alias.kind == NodeKind::Package)
        .expect("package alias")
        .alias
        .clone();
    let packet_one_path = temporary.path().join("revision-one.packet.json");
    fs::write(&packet_one_path, &context_one_output.stdout).expect("write revision-one packet");
    let rename_plan = format!(
        "plan {{ packet {} workspace {workspace} base_revision 1 operations [ \
         (rename_node {{ node (existing @{package_alias}) name demo_renamed }}) ] return_symbols [] }}",
        context_one.digest
    );
    let packet_validate_arguments = vec![
        "agent".into(),
        "validate".into(),
        "--state".into(),
        temporary.path().display().to_string(),
        "--packet".into(),
        packet_one_path.display().to_string(),
    ];
    let packet_apply_arguments = vec![
        "agent".into(),
        "apply".into(),
        "--state".into(),
        temporary.path().display().to_string(),
        "--packet".into(),
        packet_one_path.display().to_string(),
    ];
    let predicted_rename =
        transaction_receipt(&cli(&packet_validate_arguments, rename_plan.as_bytes()));
    let committed_rename =
        transaction_receipt(&cli(&packet_apply_arguments, rename_plan.as_bytes()));
    let mut expected_rename = predicted_rename;
    expected_rename.published = true;
    assert_eq!(committed_rename, expected_rename);

    let stale = cli(&packet_apply_arguments, rename_plan.as_bytes());
    assert!(
        stale.status.success(),
        "semantic rejection is a valid response"
    );
    match serde_json::from_slice::<Response>(&stale.stdout).expect("stale response") {
        Response::Error(error) => assert_eq!(error.code, ErrorCode::RevisionConflict),
        response => panic!("expected stale-base rejection, received {response:?}"),
    }

    let review_arguments = vec![
        "agent".into(),
        "context".into(),
        "--state".into(),
        temporary.path().display().to_string(),
        "--workspace".into(),
        workspace,
        "--revision".into(),
        "2".into(),
        "--purpose".into(),
        "review".into(),
        "--from-revision".into(),
        "1".into(),
    ];
    let review = cli(&review_arguments, &[]);
    assert!(review.status.success());
    let review_path = temporary.path().join("review.packet.json");
    fs::write(&review_path, &review.stdout).expect("write review packet");
    let diff = cli(
        &[
            "agent".into(),
            "diff".into(),
            "--packet".into(),
            review_path.display().to_string(),
        ],
        &[],
    );
    assert!(diff.status.success());
    let diff = String::from_utf8(diff.stdout).expect("UTF-8 diff");
    assert!(diff.contains("renamed \"demo\" -> \"demo_renamed\""));

    let malformed = cli(
        &strings(&[
            "agent",
            "validate",
            "--state",
            temporary.path().to_str().expect("UTF-8 state"),
        ]),
        b"plan { workspace }",
    );
    assert_eq!(malformed.status.code(), Some(2));
    let malformed: serde_json::Value =
        serde_json::from_slice(&malformed.stdout).expect("plan error JSON");
    assert_eq!(malformed["kind"], "plan_error");
    assert_eq!(malformed["error"]["line"], 1);

    daemon.shutdown();
}

fn mutate_boundary(source: &[u8], seed: u64, case: u64) -> Vec<u8> {
    let mut state = seed ^ case.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    let index = usize::try_from(state).unwrap_or(usize::MAX) % source.len().max(1);
    let mut mutation = source.to_vec();
    match case % 6 {
        0 if !mutation.is_empty() => mutation[index] ^= 1_u8 << (state % 8),
        1 if !mutation.is_empty() => {
            mutation.remove(index);
        }
        2 => mutation.insert(index.min(mutation.len()), b'!'),
        3 => mutation.truncate(index.min(mutation.len())),
        4 => mutation.extend_from_slice(b"{}"),
        _ if !mutation.is_empty() => mutation[index] = 0xff,
        _ => mutation.push(0xff),
    }
    mutation
}

fn run_workbench_boundary_mutation(seed: u64, cases: u64) {
    let temporary = tempfile::tempdir().expect("state directory");
    let daemon = RunningDaemon::start(temporary.path());
    let created = cli(
        &strings(&[
            "agent",
            "create",
            "--state",
            temporary.path().to_str().expect("UTF-8 state"),
        ]),
        &[],
    );
    let Response::WorkspaceCreated(created) =
        serde_json::from_slice::<Response>(&created.stdout).expect("workspace response")
    else {
        panic!("workspace response");
    };
    let workspace = created.workspace.to_string();
    let packet_bytes = cli(&packet_command(temporary.path(), &workspace, 0), &[]).stdout;
    let packet = decode_context_packet(&packet_bytes).expect("context packet");
    let root_alias = &packet.payload.aliases[0].alias;
    let plan = format!(
        "plan {{ packet {} workspace {workspace} base_revision 0 operations [ \
         (rename_node {{ node (existing @{root_alias}) name trial }}) ] return_symbols [] }}",
        packet.digest
    );
    parse_edit_plan(
        plan.as_bytes(),
        TransactionMode::ValidateOnly,
        Some(&packet),
    )
    .expect("valid mutation-plan corpus");

    for case in 0..cases {
        if case % 32 == 0 {
            let mutation = mutate_boundary(&packet_bytes, seed, case);
            assert_eq!(
                decode_context_packet(&mutation),
                decode_context_packet(&mutation),
                "packet mutation {case} is nondeterministic"
            );
        } else {
            let mutation = mutate_boundary(plan.as_bytes(), seed, case);
            assert_eq!(
                parse_edit_plan(&mutation, TransactionMode::ValidateOnly, Some(&packet)),
                parse_edit_plan(&mutation, TransactionMode::ValidateOnly, Some(&packet)),
                "plan mutation {case} is nondeterministic"
            );
        }
    }
    daemon.shutdown();
}

#[test]
fn deterministic_workbench_boundary_mutation_normal_smoke() {
    run_workbench_boundary_mutation(0x776f_726b_6265_6e63, 600);
}

/// Deterministic mutation smoke, not coverage-guided fuzzing.
///
/// Reproduce with:
/// `LKJSCRIPT_WORKBENCH_MUTATION_SEED=1 LKJSCRIPT_WORKBENCH_MUTATION_CASES=10000 cargo test --release --test workbench workbench_boundary_mutation_smoke -- --ignored --nocapture --test-threads=1`
#[test]
#[ignore = "bounded manual deterministic workbench mutation smoke"]
fn workbench_boundary_mutation_smoke() {
    let seed = std::env::var("LKJSCRIPT_WORKBENCH_MUTATION_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    let cases = std::env::var("LKJSCRIPT_WORKBENCH_MUTATION_CASES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(10_000);
    eprintln!(
        "deterministic workbench mutation smoke (not coverage-guided): seed={seed} cases={cases}"
    );
    run_workbench_boundary_mutation(seed, cases);
}
