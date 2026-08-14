#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use lkjscript::artifact;
use lkjscript::daemon;
use lkjscript::{
    Client, ErrorCode, IdempotencyKey, LocalHandle, NodeId, NodeTarget, OperationDraft, Request,
    RequestId, Response, Revision, RuntimeValue, SemanticType, Transaction, TransactionOp,
    ValueDraft, WorkspaceId,
};
use std::fs;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
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
            .expect("spawn daemon");
        let deadline = Instant::now() + Duration::from_secs(5);
        let endpoint = daemon::endpoint_path(state);
        while !endpoint.exists() {
            if let Some(status) = child.try_wait().expect("query daemon status") {
                panic!("daemon exited before readiness with {status}");
            }
            assert!(Instant::now() < deadline, "daemon readiness timed out");
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
        let response = self
            .client()
            .request(RequestId::new(9000), &Request::Shutdown)
            .expect("shutdown request");
        assert_eq!(response, Response::Acknowledged);
        let status = self.child.wait().expect("wait for daemon");
        assert!(status.success());
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

#[test]
fn source_free_durable_client_daemon_vertical_executes_old_and_new_snapshots() {
    let temporary = tempfile::tempdir().expect("temporary state directory");
    let daemon = RunningDaemon::start(temporary.path());
    let client = daemon.client();
    abandon_query_response(temporary.path());

    let cli_create = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args([
            "--state",
            temporary.path().to_str().expect("UTF-8 state path"),
            "workspace-create",
        ])
        .output()
        .expect("run documented workspace-create command");
    assert!(
        cli_create.status.success(),
        "workspace-create stderr: {}",
        String::from_utf8_lossy(&cli_create.stderr)
    );
    let create_output = String::from_utf8(cli_create.stdout).expect("UTF-8 client output");
    let workspace = create_output
        .split_whitespace()
        .find_map(|field| field.strip_prefix("workspace="))
        .expect("workspace field")
        .parse::<WorkspaceId>()
        .expect("workspace identity");
    let initial = workspace_summary(&client, workspace, Revision::INITIAL);
    assert_eq!(initial.revision, Revision::INITIAL);
    assert!(!initial.complete);

    let bootstrap = bootstrap_transaction(workspace, false);
    let applied = client
        .request(
            RequestId::new(2),
            &Request::ApplyTransaction(bootstrap.clone()),
        )
        .expect("bootstrap request");
    let Response::TransactionApplied(first) = applied else {
        panic!("unexpected transaction response: {applied:?}");
    };
    assert_eq!(first.revision, Revision::new(1));
    assert!(first.published);
    let function = allocation(&first, 3);
    let module = allocation(&first, 2);
    let constant_two = allocation(&first, 7);
    let add = allocation(&first, 8);
    assert_eq!(function.serial(), 4);

    let exact_retry = client
        .request(
            RequestId::new(3),
            &Request::ApplyTransaction(bootstrap.clone()),
        )
        .expect("idempotent retry");
    assert_eq!(exact_retry, Response::TransactionApplied(first.clone()));
    let cli_bootstrap = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args([
            "--state",
            temporary.path().to_str().expect("UTF-8 state path"),
            "bootstrap-42",
            &workspace.to_string(),
        ])
        .output()
        .expect("run documented bootstrap client command");
    assert!(
        cli_bootstrap.status.success(),
        "bootstrap stderr: {}",
        String::from_utf8_lossy(&cli_bootstrap.stderr)
    );
    assert!(String::from_utf8_lossy(&cli_bootstrap.stdout).contains("handle=3"));
    let mut conflicting = bootstrap.clone();
    if let TransactionOp::CreatePackage { name, .. } = &mut conflicting.operations[0] {
        *name = "different".to_owned();
    }
    let conflict = client
        .request(RequestId::new(4), &Request::ApplyTransaction(conflicting))
        .expect("conflicting retry response");
    assert_error(conflict, ErrorCode::IdempotencyConflict);

    let summary = workspace_summary(&client, workspace, Revision::new(1));
    assert!(summary.complete);
    assert_eq!(summary.node_count, 10);
    let node = client
        .request(
            RequestId::new(5),
            &Request::Node {
                workspace,
                revision: Revision::new(1),
                node: function,
                expand: true,
            },
        )
        .expect("function query");
    let Response::Node(view) = node else {
        panic!("unexpected node response: {node:?}");
    };
    assert_eq!(view.summary.node, function);
    assert_eq!(view.summary.display_name.as_deref(), Some("main"));
    assert_eq!(
        view.summary
            .signature
            .as_ref()
            .map(|signature| signature.result),
        Some(SemanticType::I64)
    );
    assert!(view.record.is_some());
    assert_run(&client, workspace, Revision::new(1), function, 42);

    let cli = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args([
            "--state",
            temporary.path().to_str().expect("UTF-8 state path"),
            "run",
            &workspace.to_string(),
            "1",
            &function.serial().to_string(),
        ])
        .output()
        .expect("run real client binary");
    assert!(
        cli.status.success(),
        "client stderr: {}",
        String::from_utf8_lossy(&cli.stderr)
    );
    assert!(String::from_utf8_lossy(&cli.stdout).contains("i64=42"));

    let revision_one_path = revision_path(temporary.path(), workspace, Revision::new(1));
    let revision_one_bytes = fs::read(&revision_one_path).expect("read revision artifact");
    let decoded = artifact::decode(&revision_one_bytes).expect("decode revision artifact");
    assert_eq!(
        artifact::encode(&decoded).expect("re-encode artifact"),
        revision_one_bytes
    );

    let second = Command::new(env!("CARGO_BIN_EXE_lkjscriptd"))
        .args([
            "--state",
            temporary.path().to_str().expect("UTF-8 state path"),
            "--foreground",
        ])
        .output()
        .expect("run competing daemon");
    assert!(!second.status.success());

    daemon.shutdown();
    let daemon = RunningDaemon::start(temporary.path());
    let client = daemon.client();
    assert_run(&client, workspace, Revision::new(1), function, 42);
    let persisted_retry = client
        .request(
            RequestId::new(6),
            &Request::ApplyTransaction(bootstrap.clone()),
        )
        .expect("persisted idempotent retry");
    assert_eq!(persisted_retry, Response::TransactionApplied(first.clone()));

    let edit = Transaction {
        workspace,
        base_revision: Revision::new(1),
        idempotency_key: Some(IdempotencyKey::from_bytes([0x43; 16])),
        dry_run: false,
        operations: vec![
            TransactionOp::RenameNode {
                node: NodeTarget::Existing(function),
                name: "answer".to_owned(),
            },
            TransactionOp::ReplaceOperation {
                operation: NodeTarget::Existing(constant_two),
                replacement: OperationDraft::ConstI64(3),
            },
        ],
    };
    let edited = client
        .request(RequestId::new(7), &Request::ApplyTransaction(edit))
        .expect("edit request");
    let Response::TransactionApplied(second_revision) = edited else {
        panic!("unexpected edit response: {edited:?}");
    };
    assert_eq!(second_revision.revision, Revision::new(2));
    assert!(second_revision.allocations.is_empty());
    assert_run(&client, workspace, Revision::new(1), function, 42);
    assert_run(&client, workspace, Revision::new(2), function, 43);
    let renamed = client
        .request(
            RequestId::new(8),
            &Request::Node {
                workspace,
                revision: Revision::new(2),
                node: function,
                expand: false,
            },
        )
        .expect("renamed function query");
    let Response::Node(renamed) = renamed else {
        panic!("unexpected renamed node response");
    };
    assert_eq!(renamed.summary.node, function);
    assert_eq!(renamed.summary.display_name.as_deref(), Some("answer"));

    let head_path = workspace_path(temporary.path(), workspace).join("HEAD");
    let head_before_failure = fs::read(&head_path).expect("read durable head");
    let revisions_before_failure = revision_files(temporary.path(), workspace);
    let bool_handle = LocalHandle::new(100);
    let invalid = Transaction {
        workspace,
        base_revision: Revision::new(2),
        idempotency_key: None,
        dry_run: false,
        operations: vec![
            TransactionOp::CreateOperation {
                handle: bool_handle,
                block: NodeTarget::Existing(NodeId::new(workspace, 6).expect("block id")),
                before: Some(NodeTarget::Existing(add)),
                operation: OperationDraft::ConstBool(true),
            },
            TransactionOp::ReplaceOperand {
                operation: NodeTarget::Existing(add),
                index: 1,
                value: ValueDraft::OperationResult {
                    operation: NodeTarget::Local(bool_handle),
                    output: 0,
                },
            },
        ],
    };
    let rejected = client
        .request(RequestId::new(9), &Request::ApplyTransaction(invalid))
        .expect("invalid transaction response");
    let error = assert_error(rejected, ErrorCode::TypeMismatch);
    assert_eq!(error.expected_type, Some(SemanticType::I64));
    assert_eq!(error.actual_type, Some(SemanticType::Bool));
    assert_eq!(
        workspace_summary(&client, workspace, Revision::new(2)).revision,
        Revision::new(2)
    );
    assert_eq!(
        fs::read(&head_path).expect("head after rejection"),
        head_before_failure
    );
    assert_eq!(
        revision_files(temporary.path(), workspace),
        revisions_before_failure
    );

    let create_incomplete = Transaction {
        workspace,
        base_revision: Revision::new(2),
        idempotency_key: None,
        dry_run: true,
        operations: vec![TransactionOp::CreateFunction {
            handle: LocalHandle::new(200),
            module: NodeTarget::Existing(module),
            name: "unfinished".to_owned(),
            result: SemanticType::I64,
        }],
    };
    let dry = client
        .request(
            RequestId::new(10),
            &Request::ApplyTransaction(create_incomplete.clone()),
        )
        .expect("dry-run response");
    let Response::TransactionApplied(dry) = dry else {
        panic!("unexpected dry-run response");
    };
    assert!(!dry.published);
    let future_id = allocation(&dry, 200);
    assert_eq!(future_id.serial(), 11);
    assert_eq!(
        workspace_summary(&client, workspace, Revision::new(2)).revision,
        Revision::new(2)
    );

    let mut commit_incomplete = create_incomplete;
    commit_incomplete.dry_run = false;
    let committed = client
        .request(
            RequestId::new(11),
            &Request::ApplyTransaction(commit_incomplete),
        )
        .expect("commit after dry run");
    let Response::TransactionApplied(committed) = committed else {
        panic!("unexpected commit response");
    };
    assert_eq!(allocation(&committed, 200), future_id);
    assert_eq!(committed.revision, Revision::new(3));
    assert_run(&client, workspace, Revision::new(1), function, 42);
    assert_run(&client, workspace, Revision::new(2), function, 43);

    daemon.shutdown();
    let daemon = RunningDaemon::start(temporary.path());
    let client = daemon.client();
    assert_run(&client, workspace, Revision::new(1), function, 42);
    assert_run(&client, workspace, Revision::new(2), function, 43);
    let unfinished = client
        .request(
            RequestId::new(12),
            &Request::Node {
                workspace,
                revision: Revision::new(3),
                node: future_id,
                expand: false,
            },
        )
        .expect("stable node after restart");
    assert!(matches!(unfinished, Response::Node(_)));
    let deleted = client
        .request(
            RequestId::new(13),
            &Request::ApplyTransaction(Transaction {
                workspace,
                base_revision: Revision::new(3),
                idempotency_key: None,
                dry_run: false,
                operations: vec![TransactionOp::DeleteOwnedSubtree {
                    root: NodeTarget::Existing(future_id),
                }],
            }),
        )
        .expect("durable deletion");
    assert!(matches!(deleted, Response::TransactionApplied(_)));
    daemon.shutdown();

    let daemon = RunningDaemon::start(temporary.path());
    let client = daemon.client();
    let deleted_artifact = fs::read(revision_path(temporary.path(), workspace, Revision::new(4)))
        .expect("deleted revision artifact");
    let deleted_snapshot = artifact::decode(&deleted_artifact).expect("deleted snapshot");
    assert!(deleted_snapshot.contains_tombstone(future_id.serial()));
    let old_node = client
        .request(
            RequestId::new(14),
            &Request::Node {
                workspace,
                revision: Revision::new(3),
                node: future_id,
                expand: false,
            },
        )
        .expect("old deleted node snapshot");
    assert!(matches!(old_node, Response::Node(_)));
    let replacement = client
        .request(
            RequestId::new(15),
            &Request::ApplyTransaction(Transaction {
                workspace,
                base_revision: Revision::new(4),
                idempotency_key: None,
                dry_run: false,
                operations: vec![TransactionOp::CreateFunction {
                    handle: LocalHandle::new(201),
                    module: NodeTarget::Existing(module),
                    name: "after-delete".to_owned(),
                    result: SemanticType::I64,
                }],
            }),
        )
        .expect("allocate after durable deletion");
    let Response::TransactionApplied(replacement) = replacement else {
        panic!("unexpected replacement response");
    };
    assert_eq!(allocation(&replacement, 201).serial(), 12);
    daemon.shutdown();
}

#[test]
#[ignore = "manual retained performance baseline"]
fn product_path_performance_baseline() {
    let temporary = tempfile::tempdir().expect("temporary state directory");
    let started = Instant::now();
    let daemon = RunningDaemon::start(temporary.path());
    let startup = started.elapsed().as_nanos();
    let client = daemon.client();

    let create_started = Instant::now();
    let created = client
        .request(RequestId::new(700), &Request::CreateWorkspace)
        .expect("create workspace");
    let create = create_started.elapsed().as_nanos();
    let Response::WorkspaceCreated(initial) = created else {
        panic!("unexpected create response");
    };
    let workspace = initial.workspace;
    let transaction = bootstrap_transaction(workspace, false);
    let transaction_started = Instant::now();
    let applied = client
        .request(RequestId::new(701), &Request::ApplyTransaction(transaction))
        .expect("bootstrap transaction");
    let transaction_time = transaction_started.elapsed().as_nanos();
    let Response::TransactionApplied(applied) = applied else {
        panic!("unexpected transaction response");
    };
    let entry = allocation(&applied, 3);

    let _ = workspace_summary(&client, workspace, Revision::new(1));
    assert_run(&client, workspace, Revision::new(1), entry, 42);
    let mut query_samples = Vec::new();
    let mut run_samples = Vec::new();
    let mut compile_samples = Vec::new();
    let mut execute_samples = Vec::new();
    for sample in 0..31_u64 {
        let query_started = Instant::now();
        let response = client
            .request(
                RequestId::new(800 + sample),
                &Request::WorkspaceSummary {
                    workspace,
                    revision: Revision::new(1),
                },
            )
            .expect("summary sample");
        assert!(matches!(response, Response::WorkspaceSummary(_)));
        query_samples.push(query_started.elapsed().as_nanos());
    }
    for sample in 0..31_u64 {
        let run_started = Instant::now();
        let response = client
            .request(
                RequestId::new(900 + sample),
                &Request::Run {
                    workspace,
                    revision: Revision::new(1),
                    entry,
                },
            )
            .expect("run sample");
        run_samples.push(run_started.elapsed().as_nanos());
        let Response::Run(result) = response else {
            panic!("unexpected run sample response");
        };
        assert_eq!(result.value, RuntimeValue::I64(42));
        compile_samples.push(u128::from(result.compile_nanoseconds));
        execute_samples.push(u128::from(result.execute_nanoseconds));
    }
    let artifact_size = fs::metadata(revision_path(temporary.path(), workspace, Revision::new(1)))
        .expect("artifact metadata")
        .len();
    daemon.shutdown();

    let mut restart_samples = Vec::new();
    for _ in 0..11 {
        let restart_started = Instant::now();
        let daemon = RunningDaemon::start(temporary.path());
        restart_samples.push(restart_started.elapsed().as_nanos());
        daemon.shutdown();
    }

    println!(
        "product_path_baseline samples=31 warmup=1 startup_us={:.3} create_us={:.3} transaction_us={:.3} query_median_us={:.3} query_p95_us={:.3} run_median_us={:.3} run_p95_us={:.3} compile_median_us={:.3} compile_p95_us={:.3} execute_median_us={:.3} execute_p95_us={:.3} restart_median_us={:.3} restart_p95_us={:.3} restart_samples=11 artifact_bytes={artifact_size}",
        nanos_to_micros(startup),
        nanos_to_micros(create),
        nanos_to_micros(transaction_time),
        nanos_to_micros(percentile(&query_samples, 50)),
        nanos_to_micros(percentile(&query_samples, 95)),
        nanos_to_micros(percentile(&run_samples, 50)),
        nanos_to_micros(percentile(&run_samples, 95)),
        nanos_to_micros(percentile(&compile_samples, 50)),
        nanos_to_micros(percentile(&compile_samples, 95)),
        nanos_to_micros(percentile(&execute_samples, 50)),
        nanos_to_micros(percentile(&execute_samples, 95)),
        nanos_to_micros(percentile(&restart_samples, 50)),
        nanos_to_micros(percentile(&restart_samples, 95)),
    );
}

#[test]
fn bootstrap_agent_request_cost_is_bounded_and_reproducible() {
    let workspace = WorkspaceId::from_bytes([0x21; 16]);
    let transaction = bootstrap_transaction(workspace, false);
    let transaction_bytes = lkjscript::protocol::encoded_request_size(
        RequestId::new(1),
        &Request::ApplyTransaction(transaction.clone()),
    )
    .expect("encode bootstrap request");
    let summary_bytes = lkjscript::protocol::encoded_request_size(
        RequestId::new(2),
        &Request::WorkspaceSummary {
            workspace,
            revision: Revision::new(1),
        },
    )
    .expect("encode summary request");
    let node_bytes = lkjscript::protocol::encoded_request_size(
        RequestId::new(3),
        &Request::Node {
            workspace,
            revision: Revision::new(1),
            node: NodeId::new(workspace, 4).expect("function node"),
            expand: true,
        },
    )
    .expect("encode node request");
    let run_bytes = lkjscript::protocol::encoded_request_size(
        RequestId::new(4),
        &Request::Run {
            workspace,
            revision: Revision::new(1),
            entry: NodeId::new(workspace, 4).expect("function node"),
        },
    )
    .expect("encode run request");
    assert_eq!(transaction.operations.len(), 11);
    assert!(transaction_bytes < 4096);
    println!(
        "bootstrap_agent_cost operations=11 construction_round_trips=1 first_run_round_trips=5 request_bytes={{transaction:{transaction_bytes},summary:{summary_bytes},node:{node_bytes},run:{run_bytes}}}"
    );
}

#[test]
fn explicit_hole_is_queryable_and_cannot_execute() {
    let temporary = tempfile::tempdir().expect("temporary state directory");
    let daemon = RunningDaemon::start(temporary.path());
    let client = daemon.client();
    let Response::WorkspaceCreated(initial) = client
        .request(RequestId::new(20), &Request::CreateWorkspace)
        .expect("create workspace")
    else {
        panic!("unexpected create response");
    };
    let workspace = initial.workspace;
    let transaction = bootstrap_transaction(workspace, true);
    let Response::TransactionApplied(result) = client
        .request(RequestId::new(21), &Request::ApplyTransaction(transaction))
        .expect("publish hole snapshot")
    else {
        panic!("unexpected transaction response");
    };
    let function = allocation(&result, 3);
    let block = allocation(&result, 5);
    let hole = allocation(&result, 7);
    let add = allocation(&result, 8);
    let blockers = client
        .request(
            RequestId::new(22),
            &Request::Blockers {
                workspace,
                revision: Revision::new(1),
            },
        )
        .expect("blocker query");
    let Response::Blockers { blockers, .. } = blockers else {
        panic!("unexpected blockers response");
    };
    assert!(blockers.iter().any(|blocker| {
        blocker.target == Some(hole) && blocker.expected_type == Some(SemanticType::I64)
    }));
    let run = client
        .request(
            RequestId::new(23),
            &Request::Run {
                workspace,
                revision: Revision::new(1),
                entry: function,
            },
        )
        .expect("incomplete run response");
    assert_error(run, ErrorCode::CompileIncomplete);

    let replacement = LocalHandle::new(100);
    let fill = Transaction {
        workspace,
        base_revision: Revision::new(1),
        idempotency_key: None,
        dry_run: false,
        operations: vec![
            TransactionOp::CreateOperation {
                handle: replacement,
                block: NodeTarget::Existing(block),
                before: Some(NodeTarget::Existing(add)),
                operation: OperationDraft::ConstI64(2),
            },
            TransactionOp::ReplaceOperand {
                operation: NodeTarget::Existing(add),
                index: 1,
                value: ValueDraft::OperationResult {
                    operation: NodeTarget::Local(replacement),
                    output: 0,
                },
            },
            TransactionOp::DeleteOwnedSubtree {
                root: NodeTarget::Existing(hole),
            },
        ],
    };
    let filled = client
        .request(RequestId::new(24), &Request::ApplyTransaction(fill))
        .expect("fill hole transaction");
    let Response::TransactionApplied(filled) = filled else {
        panic!("unexpected fill-hole response");
    };
    assert_eq!(filled.revision, Revision::new(2));
    assert_run(&client, workspace, Revision::new(2), function, 42);
    let old_run = client
        .request(
            RequestId::new(25),
            &Request::Run {
                workspace,
                revision: Revision::new(1),
                entry: function,
            },
        )
        .expect("old incomplete run response");
    assert_error(old_run, ErrorCode::CompileIncomplete);
    daemon.shutdown();
}

fn abandon_query_response(state: &Path) {
    let workspace = WorkspaceId::from_bytes([0xee; 16]);
    let mut body = Vec::new();
    body.extend_from_slice(&lkjscript::protocol::PROTOCOL_VERSION.to_le_bytes());
    body.extend_from_slice(&999_u64.to_le_bytes());
    body.push(3);
    body.extend_from_slice(&workspace.as_bytes());
    body.extend_from_slice(&Revision::INITIAL.get().to_le_bytes());
    let mut stream =
        UnixStream::connect(daemon::endpoint_path(state)).expect("connect abandoned client");
    stream
        .write_all(
            &u32::try_from(body.len())
                .expect("frame length")
                .to_le_bytes(),
        )
        .expect("write abandoned frame header");
    stream.write_all(&body).expect("write abandoned frame body");
    stream
        .shutdown(std::net::Shutdown::Both)
        .expect("close abandoned client");
    thread::sleep(Duration::from_millis(10));
}

fn bootstrap_transaction(workspace: WorkspaceId, hole: bool) -> Transaction {
    let package = LocalHandle::new(1);
    let module = LocalHandle::new(2);
    let function = LocalHandle::new(3);
    let region = LocalHandle::new(4);
    let block = LocalHandle::new(5);
    let forty = LocalHandle::new(6);
    let two_or_hole = LocalHandle::new(7);
    let add = LocalHandle::new(8);
    let return_operation = LocalHandle::new(9);
    let local = NodeTarget::Local;
    let result = |operation| ValueDraft::OperationResult {
        operation: local(operation),
        output: 0,
    };
    Transaction {
        workspace,
        base_revision: Revision::INITIAL,
        idempotency_key: Some(IdempotencyKey::from_bytes(if hole {
            [0x44; 16]
        } else {
            [0x42; 16]
        })),
        dry_run: false,
        operations: vec![
            TransactionOp::CreatePackage {
                handle: package,
                name: "app".to_owned(),
            },
            TransactionOp::CreateModule {
                handle: module,
                package: local(package),
                name: "root".to_owned(),
            },
            TransactionOp::CreateFunction {
                handle: function,
                module: local(module),
                name: "main".to_owned(),
                result: SemanticType::I64,
            },
            TransactionOp::CreateRegion {
                handle: region,
                function: local(function),
            },
            TransactionOp::CreateBlock {
                handle: block,
                region: local(region),
            },
            TransactionOp::CreateOperation {
                handle: forty,
                block: local(block),
                before: None,
                operation: OperationDraft::ConstI64(40),
            },
            TransactionOp::CreateOperation {
                handle: two_or_hole,
                block: local(block),
                before: None,
                operation: if hole {
                    OperationDraft::Hole {
                        expected: SemanticType::I64,
                    }
                } else {
                    OperationDraft::ConstI64(2)
                },
            },
            TransactionOp::CreateOperation {
                handle: add,
                block: local(block),
                before: None,
                operation: OperationDraft::AddI64 {
                    lhs: result(forty),
                    rhs: result(two_or_hole),
                },
            },
            TransactionOp::CreateOperation {
                handle: return_operation,
                block: local(block),
                before: None,
                operation: OperationDraft::Return { value: result(add) },
            },
            TransactionOp::SetFunctionBody {
                function: local(function),
                region: local(region),
            },
            TransactionOp::SetEntryFunction {
                package: local(package),
                function: local(function),
            },
        ],
    }
}

fn allocation(result: &lkjscript::TransactionResult, handle: u32) -> NodeId {
    result
        .allocations
        .iter()
        .find_map(|(candidate, node)| (candidate.get() == handle).then_some(*node))
        .expect("allocation handle exists")
}

fn workspace_summary(
    client: &Client,
    workspace: WorkspaceId,
    revision: Revision,
) -> lkjscript::query::WorkspaceSummary {
    let response = client
        .request(
            RequestId::new(500),
            &Request::WorkspaceSummary {
                workspace,
                revision,
            },
        )
        .expect("workspace summary response");
    let Response::WorkspaceSummary(summary) = response else {
        panic!("unexpected workspace summary: {response:?}");
    };
    summary
}

fn assert_run(
    client: &Client,
    workspace: WorkspaceId,
    revision: Revision,
    entry: NodeId,
    expected: i64,
) {
    let response = client
        .request(
            RequestId::new(600),
            &Request::Run {
                workspace,
                revision,
                entry,
            },
        )
        .expect("run response");
    let Response::Run(result) = response else {
        panic!("unexpected run response: {response:?}");
    };
    assert_eq!(result.value, RuntimeValue::I64(expected));
}

fn assert_error(response: Response, expected: ErrorCode) -> lkjscript::LkError {
    let Response::Error(error) = response else {
        panic!("expected error response, got {response:?}");
    };
    assert_eq!(error.code, expected);
    error
}

fn percentile(samples: &[u128], percentile: usize) -> u128 {
    let mut ordered = samples.to_vec();
    ordered.sort_unstable();
    let rank = ordered.len().saturating_mul(percentile).saturating_add(99) / 100;
    ordered[rank.saturating_sub(1).min(ordered.len() - 1)]
}

fn nanos_to_micros(nanoseconds: u128) -> f64 {
    nanoseconds as f64 / 1_000.0
}

fn workspace_path(state: &Path, workspace: WorkspaceId) -> PathBuf {
    state.join("workspaces").join(workspace.to_hex())
}

fn revision_path(state: &Path, workspace: WorkspaceId, revision: Revision) -> PathBuf {
    workspace_path(state, workspace)
        .join("revisions")
        .join(format!("{:020}.lkjscript", revision.get()))
}

fn revision_files(state: &Path, workspace: WorkspaceId) -> Vec<String> {
    let mut files: Vec<String> = fs::read_dir(workspace_path(state, workspace).join("revisions"))
        .expect("read revisions")
        .map(|entry| {
            entry
                .expect("revision entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    files.sort();
    files
}
