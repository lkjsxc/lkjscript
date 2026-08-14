#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use lkjscript::daemon;
use lkjscript::diff::ChangeKind;
use lkjscript::machine::{
    BoundaryErrorEnvelope, BoundaryErrorKind, JSON_ENVELOPE_VERSION, RequestEnvelope,
    ResponseEnvelope, SchemaDescription,
};
use lkjscript::query::{
    CompletenessBlocker, ContextBudget, Page, PageCursor, PageRequest, Query, QueryBatchRequest,
    QueryItem, QueryOutcome, QueryResult, RepairContext, RepairTarget,
};
use lkjscript::{
    ApplyTransactionRequest, ChangeDigest, ErrorCode, IdempotencyKey, LocalHandle, NodeId,
    NodeTarget, OperationCode, OperationDraft, OperationKind, QueryId, Request, RequestId,
    Response, Revision, RuntimeValue, SemanticType, Transaction, TransactionMode, TransactionOp,
    TransactionReceipt, TransactionResponseSpec, ValueDraft, ValueRef, WorkspaceId,
};
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct JsonDaemon {
    child: Child,
}

impl JsonDaemon {
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
            if let Some(status) = child.try_wait().expect("daemon status") {
                panic!("daemon exited before readiness: {status}");
            }
            assert!(Instant::now() < deadline, "daemon readiness timeout");
            thread::sleep(Duration::from_millis(1));
        }
        Self { child }
    }

    fn wait(mut self) {
        let status = self.child.wait().expect("wait daemon");
        assert!(status.success());
    }
}

impl Drop for JsonDaemon {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[test]
fn real_json_cli_repairs_hole_and_operand_across_restart() {
    let temporary = tempfile::tempdir().expect("state directory");
    let state = temporary.path();

    let transport = invoke_raw(
        state,
        br#"{"version":2,"request_id":1,"request":{"kind":"create_workspace"}}"#,
    );
    assert_eq!(transport.status.code(), Some(3));
    assert_one_json(&transport.stdout);
    let transport_error: BoundaryErrorEnvelope =
        serde_json::from_slice(&transport.stdout).expect("transport error JSON");
    assert_eq!(transport_error.request_id, Some(RequestId::new(1)));
    assert_eq!(transport_error.error.kind, BoundaryErrorKind::Transport);

    let daemon = JsonDaemon::start(state);
    let raw_create = invoke_raw(
        state,
        br#"{"version":2,"request_id":2,"request":{"kind":"create_workspace"}}"#,
    );
    assert!(raw_create.status.success());
    assert!(raw_create.stderr.is_empty());
    let created: ResponseEnvelope =
        serde_json::from_slice(&raw_create.stdout).expect("raw create response");
    assert_eq!(created.request_id, RequestId::new(2));
    let Response::WorkspaceCreated(initial) = created.response else {
        panic!("workspace response")
    };
    let workspace = initial.workspace;

    let head = workspace_path(state, workspace).join("HEAD");
    let head_before_validate = fs::read(&head).expect("initial HEAD");
    let validate_request = incomplete_fixture(workspace, TransactionMode::ValidateOnly, None);
    let validated = receipt(rpc(
        state,
        2,
        Request::ApplyTransaction(validate_request.clone()),
    ));
    assert!(!validated.published);
    assert_eq!(validated.revision, Revision::new(1));
    assert_eq!(validated.returned_bindings.len(), 9);
    assert_eq!(
        fs::read(&head).expect("HEAD after validate-only"),
        head_before_validate,
        "validate-only must not publish HEAD"
    );

    let key = IdempotencyKey::from_bytes([0x11; 16]);
    let commit_request = incomplete_fixture(workspace, TransactionMode::Commit, Some(key));
    let committed = receipt(rpc(
        state,
        3,
        Request::ApplyTransaction(commit_request.clone()),
    ));
    assert!(committed.published);
    assert_eq!(committed.hash, validated.hash);
    assert_eq!(committed.change_count, validated.change_count);
    assert_eq!(committed.change_digest, validated.change_digest);
    assert_eq!(committed.returned_bindings, validated.returned_bindings);
    let module = binding(&committed, 2);
    let function = binding(&committed, 3);
    let block = binding(&committed, 5);
    let forty = binding(&committed, 6);
    let two = binding(&committed, 7);
    let boolean = binding(&committed, 8);
    let hole = binding(&committed, 9);
    let return_operation = binding(&committed, 10);
    assert_eq!(committed.created_count, 10);
    assert!(!committed.complete_after);

    let invalid_page = rpc(
        state,
        41,
        Request::QueryBatch(QueryBatchRequest {
            workspace,
            revision: Revision::new(1),
            queries: vec![QueryItem {
                id: QueryId::new(1),
                query: Query::Blockers {
                    page: PageRequest {
                        after: None,
                        limit: 0,
                    },
                },
            }],
        }),
    );
    let Response::Error(invalid_page) = invalid_page.response else {
        panic!("zero-page daemon error")
    };
    assert_eq!(invalid_page.code, ErrorCode::InvalidQuery);
    let duplicate_query_ids = rpc(
        state,
        42,
        Request::QueryBatch(QueryBatchRequest {
            workspace,
            revision: Revision::new(1),
            queries: vec![
                QueryItem {
                    id: QueryId::new(1),
                    query: Query::WorkspaceSummary,
                },
                QueryItem {
                    id: QueryId::new(1),
                    query: Query::WorkspaceSummary,
                },
            ],
        }),
    );
    let Response::Error(duplicate_query_ids) = duplicate_query_ids.response else {
        panic!("duplicate query daemon error")
    };
    assert_eq!(duplicate_query_ids.code, ErrorCode::InvalidQuery);

    let (context_before, blockers) = blocker_context_batch(state, workspace, hole, 4);
    assert_eq!(context_before.expected_type, SemanticType::I64);
    assert_eq!(context_before.operation, hole);
    assert_eq!(context_before.owner_block, block);
    assert_eq!(context_before.owner_function, function);
    assert_eq!(context_before.ordinal, 3);
    assert_eq!(
        context_before
            .body_window
            .iter()
            .map(|item| item.operation)
            .collect::<Vec<_>>(),
        vec![forty, two, boolean, hole, return_operation]
    );
    assert!(context_before.visible_values.items.iter().any(|value| {
        value.producer == forty && value.compatible && value.ty == SemanticType::I64
    }));
    assert!(context_before.visible_values.items.iter().any(|value| {
        value.producer == two && value.compatible && value.ty == SemanticType::I64
    }));
    assert!(context_before.visible_values.items.iter().any(|value| {
        value.producer == boolean && !value.compatible && value.ty == SemanticType::Bool
    }));
    assert_eq!(
        context_before
            .legal_constructors
            .iter()
            .map(|constructor| constructor.code)
            .collect::<Vec<_>>(),
        vec![OperationCode::ConstI64, OperationCode::AddI64]
    );
    assert!(context_before.incoming_uses.items.iter().any(|site| {
        site.source == return_operation
            && site.operand_index == 0
            && site.expected_type == SemanticType::I64
    }));
    assert_eq!(blockers.items[0].target, Some(hole));

    let QueryResult::WorkspaceSummary(summary) = query(
        state,
        43,
        workspace,
        Revision::new(1),
        Query::WorkspaceSummary,
    ) else {
        panic!("workspace summary")
    };
    assert!(!summary.complete);
    assert_eq!(summary.blocker_count, 1);
    let body_before_refinement = collect_body(state, workspace, Revision::new(1), block, 2);
    assert_eq!(
        body_before_refinement
            .iter()
            .map(|item| item.operation)
            .collect::<Vec<_>>(),
        vec![forty, two, boolean, hole, return_operation]
    );
    let QueryResult::IncomingUses(explicit_uses) = query(
        state,
        44,
        workspace,
        Revision::new(1),
        Query::IncomingUses {
            value: ValueRef::OperationResult {
                operation: hole,
                output: 0,
            },
            page: PageRequest {
                after: None,
                limit: 1,
            },
        },
    ) else {
        panic!("incoming uses")
    };
    assert_eq!(explicit_uses.items[0].source, return_operation);
    let QueryResult::LegalConstructors(explicit_legal) = query(
        state,
        45,
        workspace,
        Revision::new(1),
        Query::LegalConstructors {
            target: RepairTarget::Hole(hole),
            include_incompatible: true,
            values: PageRequest {
                after: None,
                limit: 16,
            },
        },
    ) else {
        panic!("legal constructors")
    };
    assert_eq!(
        explicit_legal
            .constructors
            .iter()
            .map(|constructor| constructor.code)
            .collect::<Vec<_>>(),
        vec![OperationCode::ConstI64, OperationCode::AddI64]
    );

    let head_before_support_validation = fs::read(&head).expect("HEAD before support validation");
    let revisions_before_support_validation = revision_files(state, workspace);
    let support_validation = support_creation(
        workspace,
        Revision::new(1),
        TransactionMode::ValidateOnly,
        block,
        hole,
    );
    let support_prediction = receipt(rpc(
        state,
        46,
        Request::ApplyTransaction(support_validation),
    ));
    assert!(!support_prediction.published);
    let predicted_frontier = binding(&support_prediction, 100);
    assert_eq!(
        fs::read(&head).expect("HEAD after support validation"),
        head_before_support_validation
    );
    assert_eq!(
        revision_files(state, workspace),
        revisions_before_support_validation
    );

    let head_before_invalid = fs::read(&head).expect("HEAD before invalid refinement");
    let revisions_before_invalid = revision_files(state, workspace);
    let invalid_refinement = ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreateOperation {
                    handle: LocalHandle::new(100),
                    block: NodeTarget::Existing(block),
                    before: Some(NodeTarget::Existing(hole)),
                    operation: OperationDraft::ConstI64(7),
                },
                TransactionOp::RefineHole {
                    hole: NodeTarget::Existing(hole),
                    replacement: OperationDraft::AddI64 {
                        lhs: existing_result(boolean),
                        rhs: existing_result(two),
                    },
                },
            ],
        },
        response: TransactionResponseSpec {
            return_handles: vec![LocalHandle::new(100)],
        },
    };
    let invalid = rpc(state, 6, Request::ApplyTransaction(invalid_refinement));
    let Response::Error(error) = invalid.response else {
        panic!("semantic error response")
    };
    assert_eq!(error.code, ErrorCode::TypeMismatch);
    assert_eq!(error.expected_type, Some(SemanticType::I64));
    assert_eq!(error.actual_type, Some(SemanticType::Bool));
    assert_eq!(
        fs::read(&head).expect("HEAD after rejection"),
        head_before_invalid
    );
    assert_eq!(revision_files(state, workspace), revisions_before_invalid);

    let refine_key = IdempotencyKey::from_bytes([0x22; 16]);
    let valid_refinement = refinement(
        workspace,
        Some(refine_key),
        hole,
        OperationDraft::AddI64 {
            lhs: existing_result(forty),
            rhs: existing_result(two),
        },
    );
    let refined = receipt(rpc(
        state,
        7,
        Request::ApplyTransaction(valid_refinement.clone()),
    ));
    assert_eq!(refined.revision, Revision::new(2));
    assert_eq!(refined.created_count, 0);
    assert!(refined.returned_bindings.is_empty());
    assert!(refined.complete_after);
    let immediate_retry = receipt(rpc(
        state,
        47,
        Request::ApplyTransaction(valid_refinement.clone()),
    ));
    assert_eq!(immediate_retry, refined);

    let body = query(
        state,
        8,
        workspace,
        Revision::new(2),
        Query::Body {
            block,
            page: PageRequest {
                after: None,
                limit: 16,
            },
        },
    );
    let QueryResult::Body(body) = body else {
        panic!("body response")
    };
    let refined_body = body
        .items
        .iter()
        .find(|item| item.operation == hole)
        .expect("refined identity in body");
    assert_eq!(refined_body.ordinal, context_before.ordinal);
    let returned = body
        .items
        .iter()
        .find(|item| item.operation == return_operation)
        .expect("return body item");
    assert_eq!(
        returned.operands,
        vec![ValueRef::OperationResult {
            operation: hole,
            output: 0,
        }]
    );

    let diff_before_restart = collect_diff(state, workspace, Revision::new(1), Revision::new(2));
    assert_eq!(diff_before_restart.0, refined.change_count);
    assert_eq!(diff_before_restart.1, refined.change_digest);
    assert!(diff_before_restart.2.iter().any(|change| {
        change.node == hole
            && matches!(
                &change.kind,
                ChangeKind::OperationRefined {
                    before: OperationCode::Hole,
                    after: OperationCode::AddI64,
                    result_type: SemanticType::I64,
                    replacement: OperationKind::AddI64 { lhs, rhs },
                } if *lhs == ValueRef::OperationResult { operation: forty, output: 0 }
                    && *rhs == ValueRef::OperationResult { operation: two, output: 0 }
            )
    }));
    assert!(diff_before_restart.2.iter().any(|change| {
        matches!(
            change.kind,
            ChangeKind::CompletenessChanged { complete: true }
        )
    }));
    assert_run(state, 20, workspace, Revision::new(2), function, 42);

    shutdown(state, 21);
    daemon.wait();
    let daemon = JsonDaemon::start(state);
    assert_eq!(hole_context(state, workspace, hole, 22), context_before);
    assert_eq!(
        collect_diff(state, workspace, Revision::new(1), Revision::new(2)),
        diff_before_restart
    );
    let retried = receipt(rpc(
        state,
        30,
        Request::ApplyTransaction(valid_refinement.clone()),
    ));
    assert_eq!(retried, refined);
    let mut conflicting = valid_refinement.clone();
    conflicting.transaction.operations = vec![TransactionOp::RefineHole {
        hole: NodeTarget::Existing(hole),
        replacement: OperationDraft::ConstI64(9),
    }];
    let Response::Error(conflict) = rpc(state, 31, Request::ApplyTransaction(conflicting)).response
    else {
        panic!("idempotency conflict")
    };
    assert_eq!(conflict.code, ErrorCode::IdempotencyConflict);

    let head_before_operand = fs::read(&head).expect("HEAD before operand rejection");
    let revisions_before_operand = revision_files(state, workspace);
    let invalid_operand = ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::new(2),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::ReplaceOperand {
                operation: NodeTarget::Existing(hole),
                index: 1,
                value: existing_result(boolean),
            }],
        },
        response: TransactionResponseSpec::default(),
    };
    let Response::Error(operand_error) =
        rpc(state, 32, Request::ApplyTransaction(invalid_operand)).response
    else {
        panic!("operand error")
    };
    assert_eq!(operand_error.code, ErrorCode::TypeMismatch);
    assert_eq!(operand_error.expected_type, Some(SemanticType::I64));
    assert_eq!(operand_error.actual_type, Some(SemanticType::Bool));
    assert_eq!(
        fs::read(&head).expect("HEAD after operand rejection"),
        head_before_operand
    );
    assert_eq!(revision_files(state, workspace), revisions_before_operand);

    let operand_context = query(
        state,
        33,
        workspace,
        Revision::new(2),
        Query::RepairContext {
            target: RepairTarget::Operand {
                operation: hole,
                index: 1,
            },
            budget: context_budget(),
        },
    );
    let QueryResult::RepairContext(operand_context) = operand_context else {
        panic!("operand context")
    };
    assert_eq!(operand_context.operation, hole);
    assert_eq!(operand_context.operation_code, OperationCode::AddI64);
    assert_eq!(operand_context.operand_index, Some(1));
    assert_eq!(operand_context.expected_type, SemanticType::I64);
    assert_eq!(operand_context.owner_block, block);
    assert_eq!(operand_context.owner_function, function);
    assert_eq!(operand_context.ordinal, 3);
    assert_eq!(
        operand_context.current_value,
        Some(ValueRef::OperationResult {
            operation: two,
            output: 0,
        })
    );
    assert!(
        operand_context
            .body_window
            .iter()
            .any(|item| item.operation == hole && item.code == OperationCode::AddI64)
    );
    assert!(
        operand_context
            .visible_values
            .items
            .iter()
            .any(|v| v.producer == forty && v.compatible && v.ty == SemanticType::I64)
    );
    assert!(
        operand_context
            .visible_values
            .items
            .iter()
            .any(|v| v.producer == boolean && !v.compatible && v.ty == SemanticType::Bool)
    );

    let repaired = receipt(rpc(
        state,
        34,
        Request::ApplyTransaction(ApplyTransactionRequest {
            transaction: Transaction {
                workspace,
                base_revision: Revision::new(2),
                idempotency_key: Some(IdempotencyKey::from_bytes([0x33; 16])),
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::ReplaceOperand {
                    operation: NodeTarget::Existing(hole),
                    index: 1,
                    value: existing_result(forty),
                }],
            },
            response: TransactionResponseSpec::default(),
        }),
    ));
    assert_eq!(repaired.revision, Revision::new(3));
    assert_run(state, 35, workspace, Revision::new(3), function, 80);
    assert_run(state, 36, workspace, Revision::new(2), function, 42);

    let (bulk_response, compact_receipt_bytes) = rpc_observed(
        state,
        48,
        Request::ApplyTransaction(bulk_constants(workspace, block, 200)),
    );
    let bulk = receipt(bulk_response.clone());
    assert_eq!(bulk.revision, Revision::new(4));
    assert_eq!(bulk.created_count, 200);
    assert_eq!(bulk.returned_bindings.len(), 1);
    assert_eq!(binding(&bulk, 100), predicted_frontier);
    let mut all_bindings = bulk.clone();
    all_bindings.returned_bindings = (0..200_u64)
        .map(|offset| {
            (
                LocalHandle::new(100 + u32::try_from(offset).expect("bulk handle")),
                NodeId::new(workspace, predicted_frontier.serial() + offset)
                    .expect("bulk predicted node"),
            )
        })
        .collect();
    let all_binding_bytes = serde_json::to_vec(&ResponseEnvelope {
        version: JSON_ENVELOPE_VERSION,
        request_id: RequestId::new(48),
        response: Response::TransactionReceipt(all_bindings),
    })
    .expect("all-binding comparison JSON")
    .len();
    assert!(compact_receipt_bytes * 4 < all_binding_bytes);
    assert_run(state, 49, workspace, Revision::new(4), function, 80);
    assert_run(state, 50, workspace, Revision::new(3), function, 80);
    assert_run(state, 51, workspace, Revision::new(2), function, 42);

    let Response::SchemaDescription(daemon_schema) =
        rpc(state, 37, Request::DescribeSchema).response
    else {
        panic!("daemon schema")
    };
    let local_schema = local_schema();
    assert_eq!(*daemon_schema, local_schema);
    assert_eq!(module.workspace(), workspace);

    let zero_request_id = invoke_raw(
        state,
        br#"{"version":2,"request_id":0,"request":{"kind":"shutdown"}}"#,
    );
    assert_eq!(zero_request_id.status.code(), Some(2));
    let zero_request_id: BoundaryErrorEnvelope =
        serde_json::from_slice(&zero_request_id.stdout).expect("zero request ID boundary JSON");
    assert_eq!(zero_request_id.request_id, None);
    assert_eq!(zero_request_id.error.kind, BoundaryErrorKind::InvalidJson);

    let malformed = invoke_raw(
        state,
        br#"{"version":2,"request_id":99,"request":{"kind":"shutdown","unknown":true}}"#,
    );
    assert_eq!(malformed.status.code(), Some(2));
    assert_one_json(&malformed.stdout);
    let malformed: BoundaryErrorEnvelope =
        serde_json::from_slice(&malformed.stdout).expect("malformed boundary JSON");
    assert_eq!(malformed.request_id, Some(RequestId::new(99)));
    assert_eq!(malformed.error.kind, BoundaryErrorKind::InvalidJson);

    shutdown(state, 40);
    daemon.wait();
}

fn incomplete_fixture(
    workspace: WorkspaceId,
    mode: TransactionMode,
    idempotency_key: Option<IdempotencyKey>,
) -> ApplyTransactionRequest {
    let local = NodeTarget::Local;
    let result = |handle| ValueDraft::OperationResult {
        operation: local(handle),
        output: 0,
    };
    ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::INITIAL,
            idempotency_key,
            mode,
            operations: vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "app".to_owned(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: local(LocalHandle::new(1)),
                    name: "root".to_owned(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(3),
                    module: local(LocalHandle::new(2)),
                    name: "main".to_owned(),
                    result: SemanticType::I64,
                },
                TransactionOp::CreateRegion {
                    handle: LocalHandle::new(4),
                    function: local(LocalHandle::new(3)),
                },
                TransactionOp::CreateBlock {
                    handle: LocalHandle::new(5),
                    region: local(LocalHandle::new(4)),
                },
                TransactionOp::CreateOperation {
                    handle: LocalHandle::new(6),
                    block: local(LocalHandle::new(5)),
                    before: None,
                    operation: OperationDraft::ConstI64(40),
                },
                TransactionOp::CreateOperation {
                    handle: LocalHandle::new(7),
                    block: local(LocalHandle::new(5)),
                    before: None,
                    operation: OperationDraft::ConstI64(2),
                },
                TransactionOp::CreateOperation {
                    handle: LocalHandle::new(8),
                    block: local(LocalHandle::new(5)),
                    before: None,
                    operation: OperationDraft::ConstBool(true),
                },
                TransactionOp::CreateOperation {
                    handle: LocalHandle::new(9),
                    block: local(LocalHandle::new(5)),
                    before: None,
                    operation: OperationDraft::Hole {
                        expected: SemanticType::I64,
                    },
                },
                TransactionOp::CreateOperation {
                    handle: LocalHandle::new(10),
                    block: local(LocalHandle::new(5)),
                    before: None,
                    operation: OperationDraft::Return {
                        value: result(LocalHandle::new(9)),
                    },
                },
                TransactionOp::SetFunctionBody {
                    function: local(LocalHandle::new(3)),
                    region: local(LocalHandle::new(4)),
                },
                TransactionOp::SetEntryFunction {
                    package: local(LocalHandle::new(1)),
                    function: local(LocalHandle::new(3)),
                },
            ],
        },
        response: TransactionResponseSpec {
            return_handles: (2..=10).map(LocalHandle::new).collect(),
        },
    }
}

fn refinement(
    workspace: WorkspaceId,
    key: Option<IdempotencyKey>,
    hole: NodeId,
    replacement: OperationDraft,
) -> ApplyTransactionRequest {
    ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::new(1),
            idempotency_key: key,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement,
            }],
        },
        response: TransactionResponseSpec::default(),
    }
}

fn support_creation(
    workspace: WorkspaceId,
    base_revision: Revision,
    mode: TransactionMode,
    block: NodeId,
    before: NodeId,
) -> ApplyTransactionRequest {
    ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision,
            idempotency_key: None,
            mode,
            operations: vec![TransactionOp::CreateOperation {
                handle: LocalHandle::new(100),
                block: NodeTarget::Existing(block),
                before: Some(NodeTarget::Existing(before)),
                operation: OperationDraft::ConstI64(7),
            }],
        },
        response: TransactionResponseSpec {
            return_handles: vec![LocalHandle::new(100)],
        },
    }
}

fn bulk_constants(workspace: WorkspaceId, block: NodeId, count: u32) -> ApplyTransactionRequest {
    ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::new(3),
            idempotency_key: Some(IdempotencyKey::from_bytes([0x44; 16])),
            mode: TransactionMode::Commit,
            operations: (0..count)
                .map(|index| TransactionOp::CreateOperation {
                    handle: LocalHandle::new(100 + index),
                    block: NodeTarget::Existing(block),
                    before: None,
                    operation: OperationDraft::ConstI64(i64::from(index)),
                })
                .collect(),
        },
        response: TransactionResponseSpec {
            return_handles: vec![LocalHandle::new(100)],
        },
    }
}

fn existing_result(operation: NodeId) -> ValueDraft {
    ValueDraft::OperationResult {
        operation: NodeTarget::Existing(operation),
        output: 0,
    }
}

fn context_budget() -> ContextBudget {
    ContextBudget {
        body_before: 8,
        body_after: 8,
        visible_values: 16,
        incoming_uses: 16,
        include_incompatible: true,
    }
}

fn blocker_context_batch(
    state: &Path,
    workspace: WorkspaceId,
    hole: NodeId,
    request_id: u64,
) -> (RepairContext, Page<CompletenessBlocker>) {
    let response = rpc(
        state,
        request_id,
        Request::QueryBatch(QueryBatchRequest {
            workspace,
            revision: Revision::new(1),
            queries: vec![
                QueryItem {
                    id: QueryId::new(1),
                    query: Query::RepairContext {
                        target: RepairTarget::Hole(hole),
                        budget: context_budget(),
                    },
                },
                QueryItem {
                    id: QueryId::new(2),
                    query: Query::Blockers {
                        page: PageRequest {
                            after: None,
                            limit: 1,
                        },
                    },
                },
            ],
        }),
    );
    let Response::QueryBatchResult(batch) = response.response else {
        panic!("context batch")
    };
    let mut results = batch.results.into_iter();
    let QueryOutcome::Success(context) = results.next().expect("context item").outcome else {
        panic!("context outcome")
    };
    let QueryResult::RepairContext(context) = *context else {
        panic!("context result")
    };
    let QueryOutcome::Success(blockers) = results.next().expect("blocker item").outcome else {
        panic!("blocker outcome")
    };
    let QueryResult::Blockers(blockers) = *blockers else {
        panic!("blocker result")
    };
    (*context, blockers)
}

fn hole_context(state: &Path, workspace: WorkspaceId, hole: NodeId, id: u64) -> RepairContext {
    let result = query(
        state,
        id,
        workspace,
        Revision::new(1),
        Query::RepairContext {
            target: RepairTarget::Hole(hole),
            budget: context_budget(),
        },
    );
    let QueryResult::RepairContext(context) = result else {
        panic!("hole context")
    };
    *context
}

fn query(
    state: &Path,
    request_id: u64,
    workspace: WorkspaceId,
    revision: Revision,
    query: Query,
) -> QueryResult {
    let response = rpc(
        state,
        request_id,
        Request::QueryBatch(QueryBatchRequest {
            workspace,
            revision,
            queries: vec![QueryItem {
                id: QueryId::new(request_id),
                query,
            }],
        }),
    );
    let Response::QueryBatchResult(batch) = response.response else {
        panic!("query batch")
    };
    let QueryOutcome::Success(result) = batch
        .results
        .into_iter()
        .next()
        .expect("query item")
        .outcome
    else {
        panic!("query outcome")
    };
    *result
}

fn collect_body(
    state: &Path,
    workspace: WorkspaceId,
    revision: Revision,
    block: NodeId,
    limit: u32,
) -> Vec<lkjscript::query::BodyItem> {
    let mut after = None;
    let mut items = Vec::new();
    let mut request_id = 200;
    loop {
        let result = query(
            state,
            request_id,
            workspace,
            revision,
            Query::Body {
                block,
                page: PageRequest { after, limit },
            },
        );
        request_id += 1;
        let QueryResult::Body(page) = result else {
            panic!("body page")
        };
        items.extend(page.items);
        after = page.next;
        if after.is_none() {
            return items;
        }
    }
}

fn collect_diff(
    state: &Path,
    workspace: WorkspaceId,
    from: Revision,
    to: Revision,
) -> (u64, ChangeDigest, Vec<lkjscript::diff::Change>) {
    let mut after: Option<PageCursor> = None;
    let mut changes = Vec::new();
    let mut expected = None;
    let mut request_id = 100;
    loop {
        let result = query(
            state,
            request_id,
            workspace,
            to,
            Query::SemanticDiff {
                from,
                page: PageRequest { after, limit: 1 },
            },
        );
        request_id += 1;
        let QueryResult::SemanticDiff(page) = result else {
            panic!("diff page")
        };
        let facts = (page.change_count, page.change_digest);
        assert!(expected.is_none() || expected == Some(facts));
        expected = Some(facts);
        changes.extend(page.page.items);
        after = page.page.next;
        if after.is_none() {
            let (count, digest) = expected.expect("diff facts");
            assert_eq!(changes.len() as u64, count);
            return (count, digest, changes);
        }
    }
}

fn assert_run(
    state: &Path,
    request_id: u64,
    workspace: WorkspaceId,
    revision: Revision,
    entry: NodeId,
    expected: i64,
) {
    let Response::Run(run) = rpc(
        state,
        request_id,
        Request::Run {
            workspace,
            revision,
            entry,
        },
    )
    .response
    else {
        panic!("run response")
    };
    assert_eq!(run.value, RuntimeValue::I64(expected));
}

fn shutdown(state: &Path, request_id: u64) {
    assert_eq!(
        rpc(state, request_id, Request::Shutdown).response,
        Response::Acknowledged
    );
}

fn receipt(envelope: ResponseEnvelope) -> TransactionReceipt {
    let Response::TransactionReceipt(receipt) = envelope.response else {
        panic!("transaction receipt")
    };
    receipt
}

fn binding(receipt: &TransactionReceipt, handle: u32) -> NodeId {
    receipt
        .returned_bindings
        .iter()
        .find_map(|(candidate, node)| (candidate.get() == handle).then_some(*node))
        .expect("selected binding")
}

#[derive(Debug)]
struct RpcMeasurement {
    name: &'static str,
    elapsed_ns: u128,
    json_request_bytes: usize,
    json_stdout_bytes: usize,
    binary_request_bytes: usize,
    binary_response_bytes: usize,
}

fn measured_rpc(
    state: &Path,
    request_id: u64,
    name: &'static str,
    request: Request,
) -> (ResponseEnvelope, RpcMeasurement) {
    let request_id = RequestId::new(request_id);
    let input = serde_json::to_vec(&RequestEnvelope {
        version: JSON_ENVELOPE_VERSION,
        request_id,
        request: request.clone(),
    })
    .expect("encode measured request");
    let binary_request_bytes = lkjscript::protocol::encoded_request_size(request_id, &request)
        .expect("binary request size");
    let started = Instant::now();
    let output = invoke_raw(state, &input);
    let elapsed_ns = started.elapsed().as_nanos();
    assert!(output.status.success(), "measured RPC failed");
    assert!(output.stderr.is_empty());
    let response: ResponseEnvelope =
        serde_json::from_slice(&output.stdout).expect("measured response JSON");
    assert_eq!(response.request_id, request_id);
    let binary_response_bytes =
        lkjscript::protocol::encoded_response_size(request_id, &response.response)
            .expect("binary response size");
    let measurement = RpcMeasurement {
        name,
        elapsed_ns,
        json_request_bytes: input.len(),
        json_stdout_bytes: output.stdout.len(),
        binary_request_bytes,
        binary_response_bytes,
    };
    (response, measurement)
}

#[test]
#[ignore = "manual real-daemon agent repair cost measurement"]
fn agent_repair_cost_measurement() {
    let temporary = tempfile::tempdir().expect("state directory");
    let state = temporary.path();
    let daemon = JsonDaemon::start(state);
    let created = rpc(state, 1, Request::CreateWorkspace);
    let Response::WorkspaceCreated(initial) = created.response else {
        panic!("workspace response")
    };
    let workspace = initial.workspace;
    let committed = receipt(rpc(
        state,
        2,
        Request::ApplyTransaction(incomplete_fixture(
            workspace,
            TransactionMode::Commit,
            Some(IdempotencyKey::from_bytes([0x71; 16])),
        )),
    ));
    let function = binding(&committed, 3);
    let forty = binding(&committed, 6);
    let two = binding(&committed, 7);
    let boolean = binding(&committed, 8);
    let hole = binding(&committed, 9);

    let discovery_request = Request::QueryBatch(QueryBatchRequest {
        workspace,
        revision: Revision::new(1),
        queries: vec![QueryItem {
            id: QueryId::new(1),
            query: Query::Blockers {
                page: PageRequest {
                    after: None,
                    limit: 1,
                },
            },
        }],
    });
    let (_, discovery) = measured_rpc(state, 3, "blockers", discovery_request);

    let invalid = refinement(
        workspace,
        None,
        hole,
        OperationDraft::AddI64 {
            lhs: existing_result(forty),
            rhs: existing_result(boolean),
        },
    );
    let (invalid_response, invalid_measurement) =
        measured_rpc(state, 4, "invalid_edit", Request::ApplyTransaction(invalid));
    let Response::Error(invalid_error) = invalid_response.response else {
        panic!("invalid edit response")
    };
    assert_eq!(invalid_error.code, ErrorCode::TypeMismatch);

    let context_request = Request::QueryBatch(QueryBatchRequest {
        workspace,
        revision: Revision::new(1),
        queries: vec![QueryItem {
            id: QueryId::new(2),
            query: Query::RepairContext {
                target: RepairTarget::Hole(hole),
                budget: context_budget(),
            },
        }],
    });
    let (context_response, context) = measured_rpc(state, 5, "repair_context", context_request);
    let Response::QueryBatchResult(context_batch) = &context_response.response else {
        panic!("context response")
    };
    assert!(matches!(
        context_batch.results[0].outcome,
        QueryOutcome::Success(_)
    ));

    let refine_request = Request::ApplyTransaction(refinement(
        workspace,
        Some(IdempotencyKey::from_bytes([0x72; 16])),
        hole,
        OperationDraft::AddI64 {
            lhs: existing_result(forty),
            rhs: existing_result(two),
        },
    ));
    let (refine_response, refine) = measured_rpc(state, 6, "refine", refine_request);
    let Response::TransactionReceipt(refine_receipt) = &refine_response.response else {
        panic!("refinement receipt")
    };
    assert_eq!(refine_receipt.created_count, 0);
    assert!(refine_receipt.complete_after);

    let run_request = Request::Run {
        workspace,
        revision: Revision::new(2),
        entry: function,
    };
    let (run_response, run) = measured_rpc(state, 7, "run", run_request);
    let Response::Run(run_result) = run_response.response else {
        panic!("run response")
    };
    assert_eq!(run_result.value, RuntimeValue::I64(42));

    let diff_request = Request::QueryBatch(QueryBatchRequest {
        workspace,
        revision: Revision::new(2),
        queries: vec![QueryItem {
            id: QueryId::new(3),
            query: Query::SemanticDiff {
                from: Revision::new(1),
                page: PageRequest {
                    after: None,
                    limit: 256,
                },
            },
        }],
    });
    let (diff_response, diff_page) = measured_rpc(state, 8, "diff_page", diff_request);
    let Response::QueryBatchResult(diff_batch) = &diff_response.response else {
        panic!("diff response")
    };
    let QueryOutcome::Success(diff_result) = &diff_batch.results[0].outcome else {
        panic!("diff outcome")
    };
    let QueryResult::SemanticDiff(diff_result) = diff_result.as_ref() else {
        panic!("diff result")
    };
    assert!(diff_result.page.items.iter().any(|change| {
        change.node == hole && matches!(change.kind, ChangeKind::OperationRefined { .. })
    }));

    let summary_request = Request::QueryBatch(QueryBatchRequest {
        workspace,
        revision: Revision::new(2),
        queries: vec![QueryItem {
            id: QueryId::new(4),
            query: Query::WorkspaceSummary,
        }],
    });
    let (summary_response, summary_measurement) =
        measured_rpc(state, 9, "workspace_summary", summary_request);
    let Response::QueryBatchResult(summary_batch) = &summary_response.response else {
        panic!("summary response")
    };
    let QueryOutcome::Success(summary_result) = &summary_batch.results[0].outcome else {
        panic!("summary outcome")
    };
    let QueryResult::WorkspaceSummary(summary) = summary_result.as_ref() else {
        panic!("summary result")
    };
    assert_eq!(summary.node_count, 11);

    let workflow = [&context, &refine, &run];
    let total_elapsed_ns: u128 = workflow.iter().map(|item| item.elapsed_ns).sum();
    let total_json_request_bytes: usize = workflow.iter().map(|item| item.json_request_bytes).sum();
    let total_json_stdout_bytes: usize = workflow.iter().map(|item| item.json_stdout_bytes).sum();
    let total_binary_request_bytes: usize =
        workflow.iter().map(|item| item.binary_request_bytes).sum();
    let total_binary_response_bytes: usize =
        workflow.iter().map(|item| item.binary_response_bytes).sum();
    let largest_json_response_bytes = workflow
        .iter()
        .map(|item| item.json_stdout_bytes)
        .max()
        .expect("workflow response");
    let per_request = workflow
        .iter()
        .map(|item| {
            serde_json::json!({
                "name": item.name,
                "elapsed_ns": item.elapsed_ns,
                "json_request_bytes": item.json_request_bytes,
                "json_stdout_bytes": item.json_stdout_bytes,
                "binary_request_bytes": item.binary_request_bytes,
                "binary_response_bytes": item.binary_response_bytes,
            })
        })
        .collect::<Vec<_>>();
    println!(
        "AGENT_REPAIR_COST {}",
        serde_json::json!({
            "simple_workflow": {
                "requests": 3,
                "cli_invocations": 3,
                "round_trips": 3,
                "elapsed_ns": total_elapsed_ns,
                "json_request_bytes": total_json_request_bytes,
                "json_stdout_bytes": total_json_stdout_bytes,
                "binary_request_bytes": total_binary_request_bytes,
                "binary_response_bytes": total_binary_response_bytes,
                "largest_json_response_bytes": largest_json_response_bytes,
                "per_request": per_request,
                "result_i64": 42,
            },
            "separate": {
                "blockers": metric_json(&discovery),
                "invalid_edit": metric_json(&invalid_measurement),
                "diff_page": metric_json(&diff_page),
                "workspace_summary": metric_json(&summary_measurement),
                "rejected_edits_before_success_in_invalid_scenario": 1,
            },
            "context_json_stdout_bytes": context.json_stdout_bytes,
            "receipt_json_stdout_bytes": refine.json_stdout_bytes,
            "diff_page_json_stdout_bytes": diff_page.json_stdout_bytes,
            "whole_workspace_node_count": summary.node_count,
            "whole_workspace_dump_requested": false,
            "model_tokens_measured": false,
        })
    );
    shutdown(state, 10);
    daemon.wait();
}

fn metric_json(item: &RpcMeasurement) -> serde_json::Value {
    serde_json::json!({
        "name": item.name,
        "elapsed_ns": item.elapsed_ns,
        "json_request_bytes": item.json_request_bytes,
        "json_stdout_bytes": item.json_stdout_bytes,
        "binary_request_bytes": item.binary_request_bytes,
        "binary_response_bytes": item.binary_response_bytes,
    })
}

fn rpc(state: &Path, request_id: u64, request: Request) -> ResponseEnvelope {
    rpc_observed(state, request_id, request).0
}

fn rpc_observed(state: &Path, request_id: u64, request: Request) -> (ResponseEnvelope, usize) {
    let request_id = RequestId::new(request_id);
    let input = serde_json::to_vec(&RequestEnvelope {
        version: JSON_ENVELOPE_VERSION,
        request_id,
        request,
    })
    .expect("encode request");
    let output = invoke_raw(state, &input);
    assert!(
        output.status.success(),
        "rpc failed: {} / {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert!(
        output.stdout.len() < 256 * 1024,
        "fixture RPC response must stay compact"
    );
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    assert!(!output.stdout[..output.stdout.len() - 1].contains(&b'\n'));
    let response: ResponseEnvelope = serde_json::from_slice(&output.stdout).expect("response JSON");
    assert_eq!(response.version, JSON_ENVELOPE_VERSION);
    assert_eq!(response.request_id, request_id);
    (response, output.stdout.len())
}

fn invoke_raw(state: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args(["--state", state.to_str().expect("UTF-8 state path"), "rpc"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn JSON client");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input)
        .expect("write JSON request");
    child.wait_with_output().expect("JSON client output")
}

fn local_schema() -> SchemaDescription {
    let output = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .arg("schema")
        .output()
        .expect("local schema");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_one_json(&output.stdout);
    serde_json::from_slice(&output.stdout).expect("schema JSON")
}

fn assert_one_json(bytes: &[u8]) {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let _ = serde::de::IgnoredAny::deserialize(&mut deserializer).expect("one JSON value");
    deserializer.end().expect("no trailing JSON value");
}

fn workspace_path(state: &Path, workspace: WorkspaceId) -> PathBuf {
    state.join("workspaces").join(workspace.to_string())
}

fn revision_files(state: &Path, workspace: WorkspaceId) -> Vec<String> {
    let path = workspace_path(state, workspace).join("revisions");
    let mut files: Vec<String> = fs::read_dir(path)
        .expect("revision directory")
        .map(|entry| {
            entry
                .expect("revision entry")
                .file_name()
                .into_string()
                .expect("UTF-8 revision name")
        })
        .collect();
    files.sort();
    files
}
