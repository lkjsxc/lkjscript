#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use lkjscript::daemon;
use lkjscript::query::{
    ContextBudget, PageRequest, Query, QueryBatchRequest, QueryItem, QueryOutcome, QueryResult,
    RepairTarget, VisibleCursorPurpose,
};
use lkjscript::{
    ApplyTransactionRequest, Client, ErrorCode, ExpressionDraft, ExpressionKindDraft,
    FunctionBodyDraft, FunctionParameterDraft, IdempotencyKey, LocalHandle, NodeId, NodeTarget,
    OperationDraft, ProductFieldDraft, QueryId, Request, RequestId, Response, Revision,
    RuntimeFieldValue, RuntimeValue, SemanticType, Transaction, TransactionMode, TransactionOp,
    TransactionResponseSpec, TypeDraft, ValueDraft, WorkspaceId, YieldingBodyDraft,
};
use std::fs;
use std::io::{Read, Write};
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
fn real_daemon_generic_client_accepts_and_returns_canonical_nominal_value() {
    let temporary = tempfile::tempdir().expect("state");
    let daemon = RunningDaemon::start(temporary.path());
    let client = daemon.client();
    let Response::WorkspaceCreated(created) = client
        .request(RequestId::new(1), &Request::CreateWorkspace)
        .expect("create workspace")
    else {
        panic!("workspace response")
    };
    let workspace = created.workspace;
    let local = |value| NodeTarget::Local(LocalHandle::new(value));
    let request = ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateProductType {
                    handle: LocalHandle::new(3),
                    module: local(2),
                    name: "Reading".into(),
                    fields: vec![
                        ProductFieldDraft {
                            handle: LocalHandle::new(4),
                            name: "value".into(),
                            ty: TypeDraft::I64,
                        },
                        ProductFieldDraft {
                            handle: LocalHandle::new(5),
                            name: "valid".into(),
                            ty: TypeDraft::Bool,
                        },
                    ],
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(6),
                    module: local(2),
                    name: "identity".into(),
                    parameters: vec![FunctionParameterDraft {
                        handle: LocalHandle::new(7),
                        name: "reading".into(),
                        ty: TypeDraft::Nominal(local(3)),
                    }],
                    result: TypeDraft::Nominal(local(3)),
                    body: Some(FunctionBodyDraft {
                        operations: vec![],
                        return_value: ValueDraft::FunctionParameter(local(7)),
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: local(1),
                    function: local(6),
                },
            ],
        },
        response: TransactionResponseSpec {
            return_handles: vec![
                LocalHandle::new(3),
                LocalHandle::new(4),
                LocalHandle::new(5),
                LocalHandle::new(6),
            ],
        },
    };
    let Response::TransactionReceipt(receipt) = client
        .request(RequestId::new(2), &Request::ApplyTransaction(request))
        .expect("create nominal program")
    else {
        panic!("transaction response")
    };
    let id = |handle: u32| {
        receipt
            .returned_bindings
            .iter()
            .find_map(|(candidate, node)| (candidate.get() == handle).then_some(*node))
            .expect("binding")
    };
    let input = RuntimeValue::Product {
        ty: id(3),
        fields: vec![
            RuntimeFieldValue {
                field: id(5),
                value: RuntimeValue::Bool(true),
            },
            RuntimeFieldValue {
                field: id(4),
                value: RuntimeValue::I64(12),
            },
        ],
    };
    let Response::Run(result) = client
        .request(
            RequestId::new(3),
            &Request::Run {
                workspace,
                revision: Revision::new(1),
                entry: id(6),
                arguments: vec![input.clone()],
                policy: lkjscript::RunPolicy {
                    fuel: 100,
                    maximum_frames: 10,
                },
            },
        )
        .expect("nominal Run")
    else {
        panic!("Run response")
    };
    let expected = RuntimeValue::Product {
        ty: id(3),
        fields: vec![
            RuntimeFieldValue {
                field: id(4),
                value: RuntimeValue::I64(12),
            },
            RuntimeFieldValue {
                field: id(5),
                value: RuntimeValue::Bool(true),
            },
        ],
    };
    assert_eq!(result.value, expected);
    daemon.shutdown();

    let restarted = RunningDaemon::start(temporary.path());
    let Response::Run(result) = restarted
        .client()
        .request(
            RequestId::new(4),
            &Request::Run {
                workspace,
                revision: Revision::new(1),
                entry: id(6),
                arguments: vec![input],
                policy: lkjscript::RunPolicy {
                    fuel: 100,
                    maximum_frames: 10,
                },
            },
        )
        .expect("nominal Run after restart")
    else {
        panic!("Run response after restart")
    };
    assert_eq!(result.value, expected);
    restarted.shutdown();
}

fn assert_daemon_start_rejects(state: &Path, expected: &str) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lkjscriptd"))
        .args([
            "--state",
            state.to_str().expect("UTF-8 state path"),
            "--foreground",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rejecting daemon");
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().expect("query rejecting daemon") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("rejecting daemon did not exit before deadline");
        }
        thread::sleep(Duration::from_millis(1));
    };
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("rejecting daemon stderr")
        .read_to_string(&mut stderr)
        .expect("read rejecting daemon stderr");
    assert!(!status.success(), "rejecting daemon unexpectedly succeeded");
    assert!(
        stderr.contains(expected),
        "daemon rejection did not contain {expected:?}: {stderr}"
    );
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
        .request(
            RequestId::new(701),
            &Request::ApplyTransaction(apply(transaction)),
        )
        .expect("bootstrap transaction");
    let transaction_time = transaction_started.elapsed().as_nanos();
    let Response::TransactionReceipt(applied) = applied else {
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
                &query_request(workspace, Revision::new(1), Query::WorkspaceSummary),
            )
            .expect("summary sample");
        assert!(matches!(
            one_query(response),
            QueryResult::WorkspaceSummary(_)
        ));
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
                    arguments: vec![],
                    policy: lkjscript::RunPolicy {
                        fuel: 1_000_000,
                        maximum_frames: 10_000,
                    },
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
        &Request::ApplyTransaction(apply(transaction.clone())),
    )
    .expect("encode bootstrap request");
    let summary_bytes = lkjscript::protocol::encoded_request_size(
        RequestId::new(2),
        &query_request(workspace, Revision::new(1), Query::WorkspaceSummary),
    )
    .expect("encode summary request");
    let node_bytes = lkjscript::protocol::encoded_request_size(
        RequestId::new(3),
        &query_request(
            workspace,
            Revision::new(1),
            Query::Node {
                node: NodeId::new(workspace, 4).expect("function node"),
                expand: true,
            },
        ),
    )
    .expect("encode node request");
    let run_bytes = lkjscript::protocol::encoded_request_size(
        RequestId::new(4),
        &Request::Run {
            workspace,
            revision: Revision::new(1),
            entry: NodeId::new(workspace, 4).expect("function node"),
            arguments: vec![],
            policy: lkjscript::RunPolicy {
                fuel: 1_000_000,
                maximum_frames: 10_000,
            },
        },
    )
    .expect("encode run request");
    assert_eq!(transaction.operations.len(), 4);
    assert!(transaction_bytes < 4096);
    println!(
        "bootstrap_agent_cost operations=4 construction_round_trips=1 first_run_round_trips=5 request_bytes={{transaction:{transaction_bytes},summary:{summary_bytes},node:{node_bytes},run:{run_bytes}}}"
    );
}

#[test]
fn operand_repair_context_rejects_wrong_type_and_publishes_typed_repair() {
    let temporary = tempfile::tempdir().expect("temporary state directory");
    let daemon = RunningDaemon::start(temporary.path());
    let client = daemon.client();
    let Response::WorkspaceCreated(initial) = client
        .request(RequestId::new(40), &Request::CreateWorkspace)
        .expect("create workspace")
    else {
        panic!("workspace response")
    };
    let workspace = initial.workspace;
    let Response::TransactionReceipt(bootstrap) = client
        .request(
            RequestId::new(41),
            &Request::ApplyTransaction(apply(bootstrap_transaction(workspace, false))),
        )
        .expect("bootstrap")
    else {
        panic!("bootstrap response")
    };
    let function = allocation(&bootstrap, 3);
    let forty = allocation(&bootstrap, 6);
    let add = allocation(&bootstrap, 8);
    let Response::QueryBatchResult(owner_batch) = client
        .request(
            RequestId::new(46),
            &query_request(
                workspace,
                Revision::new(1),
                Query::OwnerChain {
                    node: add,
                    page: PageRequest {
                        after: None,
                        limit: 8,
                    },
                },
            ),
        )
        .expect("owner query")
    else {
        panic!("owner response")
    };
    let QueryOutcome::Success(owner_result) = &owner_batch.results[0].outcome else {
        panic!("owner outcome")
    };
    let QueryResult::OwnerChain(owners) = owner_result.as_ref() else {
        panic!("owner result")
    };
    let block = owners
        .items
        .iter()
        .find(|owner| owner.kind == lkjscript::NodeKind::Block)
        .expect("block owner")
        .node;
    assert_run(&client, workspace, Revision::new(1), function, 42);

    let Response::TransactionReceipt(with_bool) = client
        .request(
            RequestId::new(42),
            &Request::ApplyTransaction(apply(Transaction {
                workspace,
                base_revision: Revision::new(1),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::InsertExpression {
                    block,
                    before: Some(add),
                    expression: ExpressionDraft {
                        handle: LocalHandle::new(100),
                        operation: ExpressionKindDraft::ConstBool(true),
                    },
                }],
            })),
        )
        .expect("publish bool candidate")
    else {
        panic!("bool response")
    };
    let boolean = allocation(&with_bool, 100);
    assert_eq!(with_bool.revision, Revision::new(2));
    let context_response = client
        .request(
            RequestId::new(43),
            &query_request(
                workspace,
                Revision::new(2),
                Query::RepairContext {
                    target: RepairTarget::Operand {
                        operation: add,
                        index: 1,
                    },
                    budget: ContextBudget {
                        body_before: 8,
                        body_after: 1,
                        visible_values: 8,
                        incoming_uses: 8,
                        include_incompatible: true,
                    },
                },
            ),
        )
        .expect("operand context");
    let QueryResult::RepairContext(context) = one_query(context_response) else {
        panic!("operand context result")
    };
    assert_eq!(context.expected_type, SemanticType::I64);
    assert!(
        context
            .visible_values
            .items
            .iter()
            .any(|candidate| candidate.producer == forty && candidate.compatible)
    );
    assert!(
        context
            .visible_values
            .items
            .iter()
            .any(|candidate| candidate.producer == boolean && !candidate.compatible)
    );

    let head = workspace_path(temporary.path(), workspace).join("HEAD");
    let head_before = fs::read(&head).expect("head before invalid repair");
    let revisions_before = revision_files(temporary.path(), workspace);
    let rejected = client
        .request(
            RequestId::new(44),
            &Request::ApplyTransaction(apply(Transaction {
                workspace,
                base_revision: Revision::new(2),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::ReplaceOperand {
                    operation: NodeTarget::Existing(add),
                    index: 1,
                    value: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(boolean),
                        output: 0,
                    },
                }],
            })),
        )
        .expect("invalid operand repair response");
    let error = assert_error(rejected, ErrorCode::TypeMismatch);
    assert_eq!(error.expected_type, Some(SemanticType::I64));
    assert_eq!(error.actual_type, Some(SemanticType::Bool));
    assert_eq!(
        fs::read(&head).expect("head after invalid repair"),
        head_before
    );
    assert_eq!(
        revision_files(temporary.path(), workspace),
        revisions_before
    );

    let Response::TransactionReceipt(repaired) = client
        .request(
            RequestId::new(45),
            &Request::ApplyTransaction(apply(Transaction {
                workspace,
                base_revision: Revision::new(2),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::ReplaceOperand {
                    operation: NodeTarget::Existing(add),
                    index: 1,
                    value: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(forty),
                        output: 0,
                    },
                }],
            })),
        )
        .expect("valid operand repair")
    else {
        panic!("repair response")
    };
    assert_eq!(repaired.revision, Revision::new(3));
    assert_run(&client, workspace, Revision::new(3), function, 80);
    assert_run(&client, workspace, Revision::new(2), function, 42);
    assert_run(&client, workspace, Revision::new(1), function, 42);
    daemon.shutdown();
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
    let Response::TransactionReceipt(result) = client
        .request(
            RequestId::new(21),
            &Request::ApplyTransaction(apply(transaction)),
        )
        .expect("publish hole snapshot")
    else {
        panic!("unexpected transaction response");
    };
    let function = allocation(&result, 3);
    let hole = allocation(&result, 7);
    let blockers = client
        .request(
            RequestId::new(22),
            &query_request(
                workspace,
                Revision::new(1),
                Query::Blockers {
                    page: PageRequest {
                        after: None,
                        limit: 256,
                    },
                },
            ),
        )
        .expect("blocker query");
    let QueryResult::Blockers(blockers) = one_query(blockers) else {
        panic!("unexpected blockers response");
    };
    assert!(blockers.items.iter().any(|blocker| {
        blocker.target == Some(hole) && blocker.expected_type == Some(SemanticType::I64)
    }));
    let head_before_query = fs::read(workspace_path(temporary.path(), workspace).join("HEAD"))
        .expect("head before query");
    let batch = client
        .request(
            RequestId::new(29),
            &Request::QueryBatch(QueryBatchRequest {
                workspace,
                revision: Revision::new(1),
                queries: vec![
                    QueryItem {
                        id: QueryId::new(1),
                        query: Query::WorkspaceSummary,
                    },
                    QueryItem {
                        id: QueryId::new(2),
                        query: Query::Body {
                            block: hole,
                            page: PageRequest {
                                after: None,
                                limit: 1,
                            },
                        },
                    },
                ],
            }),
        )
        .expect("partial query batch");
    let Response::QueryBatchResult(batch) = batch else {
        panic!("query batch response")
    };
    assert!(matches!(batch.results[0].outcome, QueryOutcome::Success(_)));
    assert!(matches!(batch.results[1].outcome, QueryOutcome::Error(_)));
    assert_eq!(
        fs::read(workspace_path(temporary.path(), workspace).join("HEAD"))
            .expect("head after query"),
        head_before_query
    );
    let context = client
        .request(
            RequestId::new(30),
            &query_request(
                workspace,
                Revision::new(1),
                Query::RepairContext {
                    target: lkjscript::query::RepairTarget::Hole(hole),
                    budget: lkjscript::query::ContextBudget {
                        body_before: 2,
                        body_after: 2,
                        visible_values: 8,
                        incoming_uses: 8,
                        include_incompatible: true,
                    },
                },
            ),
        )
        .expect("repair context query");
    let QueryResult::RepairContext(context) = one_query(context) else {
        panic!("repair context result")
    };
    assert_eq!(context.expected_type, SemanticType::I64);
    assert_eq!(
        context.refinement_operation,
        Some(lkjscript::TransactionOpCode::RefineHole)
    );
    let revision_one_context = (*context).clone();
    let zero_context = client
        .request(
            RequestId::new(33),
            &query_request(
                workspace,
                Revision::new(1),
                Query::RepairContext {
                    target: RepairTarget::Hole(hole),
                    budget: ContextBudget {
                        body_before: 0,
                        body_after: 0,
                        visible_values: 0,
                        incoming_uses: 0,
                        include_incompatible: true,
                    },
                },
            ),
        )
        .expect("zero-budget repair context");
    let QueryResult::RepairContext(zero_context) = one_query(zero_context) else {
        panic!("zero context result")
    };
    let visible_cursor = zero_context
        .visible_values
        .next
        .expect("zero visible cursor");
    let continued = client
        .request(
            RequestId::new(34),
            &query_request(
                workspace,
                Revision::new(1),
                Query::VisibleValues {
                    purpose: VisibleCursorPurpose::RepairContext,
                    target: RepairTarget::Hole(hole),
                    include_incompatible: true,
                    page: PageRequest {
                        after: Some(visible_cursor),
                        limit: 1,
                    },
                },
            ),
        )
        .expect("continue context visible values");
    let QueryResult::VisibleValues(continued) = one_query(continued) else {
        panic!("continued visible values")
    };
    assert_eq!(continued.items.len(), 1);
    let incoming_cursor = zero_context
        .incoming_uses
        .next
        .expect("zero incoming cursor");
    let continued_uses = client
        .request(
            RequestId::new(35),
            &query_request(
                workspace,
                Revision::new(1),
                Query::IncomingUses {
                    value: lkjscript::ValueRef::OperationResult {
                        operation: hole,
                        output: 0,
                    },
                    page: PageRequest {
                        after: Some(incoming_cursor),
                        limit: 1,
                    },
                },
            ),
        )
        .expect("continue context incoming uses");
    let QueryResult::IncomingUses(continued_uses) = one_query(continued_uses) else {
        panic!("continued uses")
    };
    assert_eq!(continued_uses.items.len(), 1);
    let run = client
        .request(
            RequestId::new(23),
            &Request::Run {
                workspace,
                revision: Revision::new(1),
                entry: function,
                arguments: vec![],
                policy: lkjscript::RunPolicy {
                    fuel: 1_000_000,
                    maximum_frames: 10_000,
                },
            },
        )
        .expect("incomplete run response");
    assert_error(run, ErrorCode::CompileIncomplete);

    let fill = Transaction {
        workspace,
        base_revision: Revision::new(1),
        idempotency_key: Some(IdempotencyKey::from_bytes([0x45; 16])),
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::RefineHole {
            hole: NodeTarget::Existing(hole),
            replacement: OperationDraft::ConstI64(2),
        }],
    };
    let filled = client
        .request(
            RequestId::new(24),
            &Request::ApplyTransaction(apply(fill.clone())),
        )
        .expect("fill hole transaction");
    let Response::TransactionReceipt(filled) = filled else {
        panic!("unexpected fill-hole response");
    };
    assert_eq!(filled.revision, Revision::new(2));
    assert_eq!(filled.created_count, 0);
    assert!(!filled.complete_before);
    assert!(filled.complete_after);
    let revision_diff = collect_diff(&client, workspace, Revision::new(1), Revision::new(2));
    assert_eq!(revision_diff.0, filled.change_count);
    assert_eq!(revision_diff.1, filled.change_digest);
    assert_run(&client, workspace, Revision::new(2), function, 42);
    let refined = client
        .request(
            RequestId::new(26),
            &query_request(
                workspace,
                Revision::new(2),
                Query::Node {
                    node: hole,
                    expand: true,
                },
            ),
        )
        .expect("refined node response");
    let QueryResult::Node(refined) = one_query(refined) else {
        panic!("unexpected refined node response");
    };
    assert_eq!(refined.summary.node, hole);
    assert!(matches!(
        refined.record,
        Some(lkjscript::Node::Operation {
            operation: lkjscript::OperationKind::ConstI64(2),
            ..
        })
    ));
    let old_run = client
        .request(
            RequestId::new(25),
            &Request::Run {
                workspace,
                revision: Revision::new(1),
                entry: function,
                arguments: vec![],
                policy: lkjscript::RunPolicy {
                    fuel: 1_000_000,
                    maximum_frames: 10_000,
                },
            },
        )
        .expect("old incomplete run response");
    assert_error(old_run, ErrorCode::CompileIncomplete);
    daemon.shutdown();

    let daemon = RunningDaemon::start(temporary.path());
    let client = daemon.client();
    assert_run(&client, workspace, Revision::new(2), function, 42);
    let restarted_context = client
        .request(
            RequestId::new(32),
            &query_request(
                workspace,
                Revision::new(1),
                Query::RepairContext {
                    target: RepairTarget::Hole(hole),
                    budget: ContextBudget {
                        body_before: 2,
                        body_after: 2,
                        visible_values: 8,
                        incoming_uses: 8,
                        include_incompatible: true,
                    },
                },
            ),
        )
        .expect("revision-one context after restart");
    let QueryResult::RepairContext(restarted_context) = one_query(restarted_context) else {
        panic!("repair context after restart")
    };
    assert_eq!(*restarted_context, revision_one_context);
    assert_eq!(
        collect_diff(&client, workspace, Revision::new(1), Revision::new(2)),
        revision_diff
    );
    let retry = client
        .request(RequestId::new(27), &Request::ApplyTransaction(apply(fill)))
        .expect("refinement retry after restart");
    assert_eq!(retry, Response::TransactionReceipt(filled));
    let old_hole = client
        .request(
            RequestId::new(28),
            &query_request(
                workspace,
                Revision::new(1),
                Query::Node {
                    node: hole,
                    expand: true,
                },
            ),
        )
        .expect("old hole after restart");
    let QueryResult::Node(old_hole) = one_query(old_hole) else {
        panic!("unexpected old hole response");
    };
    assert!(matches!(
        old_hole.record,
        Some(lkjscript::Node::Operation {
            operation: lkjscript::OperationKind::Hole { .. },
            ..
        })
    ));
    daemon.shutdown();
}

#[test]
fn real_daemon_executes_repaired_structured_program_across_restart() {
    let temporary = tempfile::tempdir().expect("temporary state directory");
    let daemon = RunningDaemon::start(temporary.path());
    assert_daemon_start_rejects(temporary.path(), "WorkspaceExists");
    let client = daemon.client();
    let Response::WorkspaceCreated(initial) = client
        .request(RequestId::new(500), &Request::CreateWorkspace)
        .expect("create")
    else {
        panic!("create response")
    };
    let workspace = initial.workspace;
    let local = |handle| NodeTarget::Local(LocalHandle::new(handle));
    let result = |handle| ValueDraft::OperationResult {
        operation: local(handle),
        output: 0,
    };
    let parameter = |handle| ValueDraft::FunctionParameter(local(handle));
    let transaction = Transaction {
        workspace,
        base_revision: Revision::INITIAL,
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![
            TransactionOp::CreatePackage {
                handle: LocalHandle::new(1),
                name: "app".into(),
            },
            TransactionOp::CreateModule {
                handle: LocalHandle::new(2),
                package: local(1),
                name: "root".into(),
            },
            TransactionOp::CreateFunction {
                handle: LocalHandle::new(10),
                module: local(2),
                name: "range_sum".into(),
                parameters: vec![FunctionParameterDraft {
                    handle: LocalHandle::new(11),
                    name: "end".into(),
                    ty: SemanticType::I64.into(),
                }],
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: vec![
                        ExpressionDraft {
                            handle: LocalHandle::new(12),
                            operation: ExpressionKindDraft::ConstI64(0),
                        },
                        ExpressionDraft {
                            handle: LocalHandle::new(13),
                            operation: ExpressionKindDraft::ForI64 {
                                start: result(12),
                                end_exclusive: parameter(11),
                                step: 1,
                                initial: result(12),
                                carried: SemanticType::I64.into(),
                                index_handle: LocalHandle::new(14),
                                carried_handle: LocalHandle::new(15),
                                body: YieldingBodyDraft {
                                    operations: vec![ExpressionDraft {
                                        handle: LocalHandle::new(16),
                                        operation: ExpressionKindDraft::Hole {
                                            expected: SemanticType::I64.into(),
                                        },
                                    }],
                                    yield_value: result(16),
                                },
                            },
                        },
                    ],
                    return_value: result(13),
                }),
            },
            TransactionOp::CreateFunction {
                handle: LocalHandle::new(20),
                module: local(2),
                name: "normalize_and_sum".into(),
                parameters: vec![FunctionParameterDraft {
                    handle: LocalHandle::new(21),
                    name: "n".into(),
                    ty: SemanticType::I64.into(),
                }],
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: vec![
                        ExpressionDraft {
                            handle: LocalHandle::new(22),
                            operation: ExpressionKindDraft::ConstI64(0),
                        },
                        ExpressionDraft {
                            handle: LocalHandle::new(23),
                            operation: ExpressionKindDraft::LtI64 {
                                lhs: parameter(21),
                                rhs: result(22),
                            },
                        },
                        ExpressionDraft {
                            handle: LocalHandle::new(24),
                            operation: ExpressionKindDraft::If {
                                condition: result(23),
                                result: SemanticType::I64.into(),
                                then_body: YieldingBodyDraft {
                                    operations: vec![],
                                    yield_value: result(22),
                                },
                                else_body: YieldingBodyDraft {
                                    operations: vec![ExpressionDraft {
                                        handle: LocalHandle::new(25),
                                        operation: ExpressionKindDraft::Call {
                                            function: local(10),
                                            arguments: vec![parameter(21)],
                                        },
                                    }],
                                    yield_value: result(25),
                                },
                            },
                        },
                    ],
                    return_value: result(24),
                }),
            },
            TransactionOp::CreateFunction {
                handle: LocalHandle::new(30),
                module: local(2),
                name: "main".into(),
                parameters: vec![],
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: vec![
                        ExpressionDraft {
                            handle: LocalHandle::new(31),
                            operation: ExpressionKindDraft::ConstI64(101),
                        },
                        ExpressionDraft {
                            handle: LocalHandle::new(32),
                            operation: ExpressionKindDraft::Call {
                                function: local(10),
                                arguments: vec![result(31)],
                            },
                        },
                    ],
                    return_value: result(32),
                }),
            },
            TransactionOp::SetEntryFunction {
                package: local(1),
                function: local(30),
            },
        ],
    };
    let Response::TransactionReceipt(created) = client
        .request(
            RequestId::new(501),
            &Request::ApplyTransaction(apply(transaction)),
        )
        .expect("create program")
    else {
        panic!("receipt")
    };
    let range = allocation(&created, 10);
    let normalize = allocation(&created, 20);
    let main = allocation(&created, 30);
    let main_call = allocation(&created, 32);
    let hole = allocation(&created, 16);
    let index = allocation(&created, 14);
    let carried = allocation(&created, 15);
    assert!(!created.complete_after);
    let refine = Transaction {
        workspace,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::RefineHole {
            hole: NodeTarget::Existing(hole),
            replacement: OperationDraft::AddI64 {
                lhs: ValueDraft::BlockArgument(NodeTarget::Existing(carried)),
                rhs: ValueDraft::BlockArgument(NodeTarget::Existing(index)),
            },
        }],
    };
    let Response::TransactionReceipt(refined) = client
        .request(
            RequestId::new(502),
            &Request::ApplyTransaction(apply(refine)),
        )
        .expect("refine")
    else {
        panic!("refine receipt")
    };
    assert!(refined.complete_after);
    assert_eq!(refined.created_count, 0);
    let rename = Transaction {
        workspace,
        base_revision: Revision::new(2),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::RenameNode {
            node: NodeTarget::Existing(range),
            name: "renamed_range_sum".into(),
        }],
    };
    let Response::TransactionReceipt(renamed) = client
        .request(
            RequestId::new(508),
            &Request::ApplyTransaction(apply(rename)),
        )
        .expect("rename")
    else {
        panic!("rename receipt")
    };
    assert_eq!(renamed.created_count, 0);
    let request_run = |request_id, entry, arguments, policy| {
        client
            .request(
                RequestId::new(request_id),
                &Request::Run {
                    workspace,
                    revision: Revision::new(3),
                    entry,
                    arguments,
                    policy,
                },
            )
            .expect("run response")
    };
    assert_eq!(
        assert_error(
            request_run(
                510,
                main,
                vec![RuntimeValue::I64(1)],
                lkjscript::RunPolicy {
                    fuel: 100,
                    maximum_frames: 10
                }
            ),
            ErrorCode::RunArgumentMismatch
        )
        .target,
        Some(main)
    );
    assert_eq!(
        assert_error(
            request_run(
                511,
                normalize,
                vec![RuntimeValue::Bool(true)],
                lkjscript::RunPolicy {
                    fuel: 100,
                    maximum_frames: 10
                }
            ),
            ErrorCode::RunArgumentMismatch
        )
        .actual_type,
        Some(SemanticType::Bool)
    );
    assert_error(
        request_run(
            512,
            main,
            vec![],
            lkjscript::RunPolicy {
                fuel: 0,
                maximum_frames: 10,
            },
        ),
        ErrorCode::PolicyExceeded,
    );
    let fuel = assert_error(
        request_run(
            513,
            main,
            vec![],
            lkjscript::RunPolicy {
                fuel: 1,
                maximum_frames: 10,
            },
        ),
        ErrorCode::ExecutionFuelExhausted,
    );
    assert_eq!(fuel.target, Some(main_call));
    let frames = assert_error(
        request_run(
            514,
            main,
            vec![],
            lkjscript::RunPolicy {
                fuel: 100,
                maximum_frames: 1,
            },
        ),
        ErrorCode::ExecutionFrameExhausted,
    );
    assert_eq!(frames.target, Some(main_call));

    let run = |client: &Client, request_id, entry, arguments| {
        let response = client
            .request(
                RequestId::new(request_id),
                &Request::Run {
                    workspace,
                    revision: Revision::new(3),
                    entry,
                    arguments,
                    policy: lkjscript::RunPolicy {
                        fuel: 1_000_000,
                        maximum_frames: 10_000,
                    },
                },
            )
            .expect("run");
        let Response::Run(result) = response else {
            panic!("run response")
        };
        result.value
    };
    assert_eq!(run(&client, 503, main, vec![]), RuntimeValue::I64(5050));
    assert_eq!(
        run(&client, 504, normalize, vec![RuntimeValue::I64(-3)]),
        RuntimeValue::I64(0)
    );
    assert_eq!(
        run(&client, 505, normalize, vec![RuntimeValue::I64(11)]),
        RuntimeValue::I64(55)
    );
    assert_eq!(
        run(&client, 506, range, vec![RuntimeValue::I64(5)]),
        RuntimeValue::I64(10)
    );
    let envelope = lkjscript::machine::RequestEnvelope {
        version: lkjscript::machine::JSON_ENVELOPE_VERSION,
        request_id: RequestId::new(509),
        request: Request::Run {
            workspace,
            revision: Revision::new(3),
            entry: main,
            arguments: vec![],
            policy: lkjscript::RunPolicy {
                fuel: 1_000_000,
                maximum_frames: 10_000,
            },
        },
    };
    let mut cli = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args([
            "--state",
            temporary.path().to_str().expect("state path"),
            "rpc",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn generic CLI");
    cli.stdin
        .as_mut()
        .expect("CLI stdin")
        .write_all(&serde_json::to_vec(&envelope).expect("request JSON"))
        .expect("write CLI request");
    let output = cli.wait_with_output().expect("CLI output");
    assert!(
        output.status.success(),
        "CLI stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: lkjscript::machine::ResponseEnvelope =
        serde_json::from_slice(&output.stdout).expect("response JSON");
    let Response::Run(result) = response.response else {
        panic!("CLI run response")
    };
    assert_eq!(result.value, RuntimeValue::I64(5050));
    daemon.shutdown();
    let restarted = RunningDaemon::start(temporary.path());
    assert_eq!(
        run(&restarted.client(), 507, main, vec![]),
        RuntimeValue::I64(5050)
    );
    restarted.shutdown();

    let repaired_revision = revision_path(temporary.path(), workspace, Revision::new(2));
    let mut corrupt = fs::read(&repaired_revision).expect("read repaired structured revision");
    *corrupt.last_mut().expect("structured revision hash byte") ^= 1;
    fs::write(&repaired_revision, corrupt).expect("corrupt repaired structured revision");
    assert_daemon_start_rejects(temporary.path(), "ArtifactCorrupt");
}

fn bootstrap_transaction(workspace: WorkspaceId, hole: bool) -> Transaction {
    let package = LocalHandle::new(1);
    let module = LocalHandle::new(2);
    let function = LocalHandle::new(3);
    let forty = LocalHandle::new(6);
    let two_or_hole = LocalHandle::new(7);
    let add = LocalHandle::new(8);
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
        mode: TransactionMode::Commit,
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
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: vec![
                        ExpressionDraft {
                            handle: forty,
                            operation: ExpressionKindDraft::ConstI64(40),
                        },
                        ExpressionDraft {
                            handle: two_or_hole,
                            operation: if hole {
                                ExpressionKindDraft::Hole {
                                    expected: SemanticType::I64.into(),
                                }
                            } else {
                                ExpressionKindDraft::ConstI64(2)
                            },
                        },
                        ExpressionDraft {
                            handle: add,
                            operation: ExpressionKindDraft::AddI64 {
                                lhs: result(forty),
                                rhs: result(two_or_hole),
                            },
                        },
                    ],
                    return_value: result(add),
                }),
            },
            TransactionOp::SetEntryFunction {
                package: local(package),
                function: local(function),
            },
        ],
    }
}

fn apply(transaction: Transaction) -> ApplyTransactionRequest {
    let mut return_handles = Vec::new();
    let mut expressions = Vec::new();
    for operation in &transaction.operations {
        if let Some(handle) = operation.created_handle() {
            return_handles.push(handle);
        }
        match operation {
            TransactionOp::CreateFunction {
                parameters, body, ..
            } => {
                return_handles.extend(parameters.iter().map(|parameter| parameter.handle));
                if let Some(body) = body {
                    expressions.extend(body.operations.iter());
                }
            }
            TransactionOp::DefineFunctionBody { body, .. } => {
                expressions.extend(body.operations.iter())
            }
            TransactionOp::InsertExpression { expression, .. } => expressions.push(expression),
            _ => {}
        }
    }
    while let Some(expression) = expressions.pop() {
        return_handles.push(expression.handle);
        match &expression.operation {
            ExpressionKindDraft::If {
                then_body,
                else_body,
                ..
            } => {
                expressions.extend(then_body.operations.iter());
                expressions.extend(else_body.operations.iter());
            }
            ExpressionKindDraft::ForI64 {
                index_handle,
                carried_handle,
                body,
                ..
            } => {
                return_handles.push(*index_handle);
                return_handles.push(*carried_handle);
                expressions.extend(body.operations.iter());
            }
            _ => {}
        }
    }
    return_handles.sort();
    return_handles.dedup();
    ApplyTransactionRequest {
        transaction,
        response: TransactionResponseSpec { return_handles },
    }
}

fn allocation(result: &lkjscript::TransactionReceipt, handle: u32) -> NodeId {
    result
        .returned_bindings
        .iter()
        .find_map(|(candidate, node)| (candidate.get() == handle).then_some(*node))
        .expect("allocation handle exists")
}

fn query_request(workspace: WorkspaceId, revision: Revision, query: Query) -> Request {
    Request::QueryBatch(QueryBatchRequest {
        workspace,
        revision,
        queries: vec![QueryItem {
            id: QueryId::new(1),
            query,
        }],
    })
}
fn one_query(response: Response) -> QueryResult {
    let Response::QueryBatchResult(mut batch) = response else {
        panic!("unexpected query response")
    };
    let item = batch.results.pop().expect("one result");
    match item.outcome {
        QueryOutcome::Success(result) => *result,
        QueryOutcome::Error(error) => panic!("query error: {error}"),
    }
}
fn collect_diff(
    client: &Client,
    workspace: WorkspaceId,
    from: Revision,
    to: Revision,
) -> (u64, lkjscript::ChangeDigest, Vec<lkjscript::diff::Change>) {
    let mut after = None;
    let mut changes = Vec::new();
    let mut metadata = None;
    loop {
        let response = client
            .request(
                RequestId::new(510),
                &query_request(
                    workspace,
                    to,
                    Query::SemanticDiff {
                        from,
                        page: PageRequest { after, limit: 1 },
                    },
                ),
            )
            .expect("diff page");
        let QueryResult::SemanticDiff(diff) = one_query(response) else {
            panic!("semantic diff result")
        };
        let current = (diff.change_count, diff.change_digest);
        assert!(metadata.is_none_or(|expected| expected == current));
        metadata = Some(current);
        changes.extend(diff.page.items);
        after = diff.page.next;
        if after.is_none() {
            break;
        }
    }
    let (count, digest) = metadata.expect("at least one diff page");
    assert_eq!(changes.len() as u64, count);
    (count, digest, changes)
}

fn workspace_summary(
    client: &Client,
    workspace: WorkspaceId,
    revision: Revision,
) -> lkjscript::query::WorkspaceSummary {
    let response = client
        .request(
            RequestId::new(500),
            &query_request(workspace, revision, Query::WorkspaceSummary),
        )
        .expect("workspace summary response");
    let QueryResult::WorkspaceSummary(summary) = one_query(response) else {
        panic!("unexpected workspace summary")
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
                arguments: vec![],
                policy: lkjscript::RunPolicy {
                    fuel: 1_000_000,
                    maximum_frames: 10_000,
                },
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
