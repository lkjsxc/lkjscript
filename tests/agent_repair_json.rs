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
    ApplyTransactionRequest, BlockArgumentRole, ChangeDigest, ErrorCode, ExpressionDraft,
    ExpressionKindDraft, FunctionBodyDraft, FunctionParameterDraft, IdempotencyKey, LocalHandle,
    NodeId, NodeTarget, OperationCode, OperationDraft, OperationKind, QueryId, RegionRole, Request,
    RequestId, Response, Revision, RuntimeValue, SemanticType, Transaction, TransactionMode,
    TransactionOp, TransactionReceipt, TransactionResponseSpec, ValueDraft, ValueRef, WorkspaceId,
    YieldingBodyDraft,
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
        br#"{"version":3,"request_id":1,"request":{"kind":"create_workspace"}}"#,
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
        br#"{"version":3,"request_id":2,"request":{"kind":"create_workspace"}}"#,
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
    assert_eq!(validated.returned_bindings.len(), 6);
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
    let structured_retry = receipt(rpc(
        state,
        49,
        Request::ApplyTransaction(commit_request.clone()),
    ));
    assert_eq!(structured_retry, committed);
    let module = binding(&committed, 2);
    let function = binding(&committed, 3);
    let forty = binding(&committed, 6);
    let two = binding(&committed, 7);
    let boolean = binding(&committed, 8);
    let hole = binding(&committed, 9);
    let QueryResult::OwnerChain(owners) = query(
        state,
        47,
        workspace,
        Revision::new(1),
        Query::OwnerChain {
            node: hole,
            page: PageRequest {
                after: None,
                limit: 8,
            },
        },
    ) else {
        panic!("owner chain")
    };
    let block = owners
        .items
        .iter()
        .find(|owner| owner.kind == lkjscript::NodeKind::Block)
        .expect("block owner")
        .node;
    let QueryResult::IncomingUses(uses) = query(
        state,
        48,
        workspace,
        Revision::new(1),
        Query::IncomingUses {
            value: ValueRef::OperationResult {
                operation: hole,
                output: 0,
            },
            page: PageRequest {
                after: None,
                limit: 8,
            },
        },
    ) else {
        panic!("incoming uses")
    };
    let return_operation = uses.items[0].source;
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
        vec![
            OperationCode::ConstI64,
            OperationCode::AddI64,
            OperationCode::Call,
            OperationCode::If,
            OperationCode::ForI64,
        ]
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
            constructors: PageRequest {
                after: None,
                limit: 16,
            },
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
            .items
            .iter()
            .map(|constructor| constructor.code)
            .collect::<Vec<_>>(),
        vec![
            OperationCode::ConstI64,
            OperationCode::AddI64,
            OperationCode::Call,
            OperationCode::If,
            OperationCode::ForI64,
        ]
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
                TransactionOp::InsertExpression {
                    block,
                    before: Some(hole),
                    expression: ExpressionDraft {
                        handle: LocalHandle::new(100),
                        operation: ExpressionKindDraft::ConstI64(7),
                    },
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
        br#"{"version":3,"request_id":0,"request":{"kind":"shutdown"}}"#,
    );
    assert_eq!(zero_request_id.status.code(), Some(2));
    let zero_request_id: BoundaryErrorEnvelope =
        serde_json::from_slice(&zero_request_id.stdout).expect("zero request ID boundary JSON");
    assert_eq!(zero_request_id.request_id, None);
    assert_eq!(zero_request_id.error.kind, BoundaryErrorKind::InvalidJson);

    let malformed = invoke_raw(
        state,
        br#"{"version":3,"request_id":99,"request":{"kind":"shutdown","unknown":true}}"#,
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
                    parameters: Vec::new(),
                    result: SemanticType::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                handle: LocalHandle::new(6),
                                operation: ExpressionKindDraft::ConstI64(40),
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(7),
                                operation: ExpressionKindDraft::ConstI64(2),
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(8),
                                operation: ExpressionKindDraft::ConstBool(true),
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(9),
                                operation: ExpressionKindDraft::Hole {
                                    expected: SemanticType::I64,
                                },
                            },
                        ],
                        return_value: result(LocalHandle::new(9)),
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: local(LocalHandle::new(1)),
                    function: local(LocalHandle::new(3)),
                },
            ],
        },
        response: TransactionResponseSpec {
            return_handles: [2, 3, 6, 7, 8, 9]
                .into_iter()
                .map(LocalHandle::new)
                .collect(),
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
            operations: vec![TransactionOp::InsertExpression {
                block,
                before: Some(before),
                expression: ExpressionDraft {
                    handle: LocalHandle::new(100),
                    operation: ExpressionKindDraft::ConstI64(7),
                },
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
                .map(|index| TransactionOp::InsertExpression {
                    block,
                    before: None,
                    expression: ExpressionDraft {
                        handle: LocalHandle::new(100 + index),
                        operation: ExpressionKindDraft::ConstI64(i64::from(index)),
                    },
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
            arguments: vec![],
            policy: lkjscript::RunPolicy {
                fuel: 1_000_000,
                maximum_frames: 10_000,
            },
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

fn structured_creation(workspace: WorkspaceId) -> ApplyTransactionRequest {
    let local = |handle| NodeTarget::Local(LocalHandle::new(handle));
    let result = |handle| ValueDraft::OperationResult {
        operation: local(handle),
        output: 0,
    };
    let parameter = |handle| ValueDraft::FunctionParameter(local(handle));
    ApplyTransactionRequest {
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
                    name: "structured".into(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(10),
                    module: local(2),
                    name: "range_sum".into(),
                    parameters: vec![FunctionParameterDraft {
                        handle: LocalHandle::new(11),
                        name: "n".into(),
                        ty: SemanticType::I64,
                    }],
                    result: SemanticType::I64,
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
                                    carried: SemanticType::I64,
                                    index_handle: LocalHandle::new(14),
                                    carried_handle: LocalHandle::new(15),
                                    body: YieldingBodyDraft {
                                        operations: vec![ExpressionDraft {
                                            handle: LocalHandle::new(16),
                                            operation: ExpressionKindDraft::Hole {
                                                expected: SemanticType::I64,
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
                        ty: SemanticType::I64,
                    }],
                    result: SemanticType::I64,
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
                                    result: SemanticType::I64,
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
                    result: SemanticType::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                handle: LocalHandle::new(31),
                                operation: ExpressionKindDraft::ConstI64(101),
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(32),
                                operation: ExpressionKindDraft::Call {
                                    function: local(20),
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
        },
        response: TransactionResponseSpec {
            return_handles: vec![
                LocalHandle::new(10),
                LocalHandle::new(16),
                LocalHandle::new(20),
                LocalHandle::new(30),
            ],
        },
    }
}

#[test]
fn real_json_cli_structured_program_repair_vertical() {
    let temporary = tempfile::tempdir().expect("state directory");
    let state = temporary.path();
    let daemon = JsonDaemon::start(state);
    let Response::WorkspaceCreated(initial) = rpc(state, 500, Request::CreateWorkspace).response
    else {
        panic!("workspace")
    };
    let workspace = initial.workspace;
    let creation = structured_creation(workspace);
    assert_eq!(creation.transaction.operations.len(), 6);
    let created = receipt(rpc(state, 501, Request::ApplyTransaction(creation)));
    assert_eq!(created.returned_bindings.len(), 4);
    assert_eq!(created.created_count, 36);
    assert!(!created.complete_after);
    let range = binding(&created, 10);
    let hole = binding(&created, 16);
    let normalize = binding(&created, 20);
    let main = binding(&created, 30);
    let revision_one_files = revision_files(state, workspace);
    assert_eq!(revision_one_files.len(), 2);

    let context = hole_context(state, workspace, hole, 502);
    assert_eq!(context.operation, hole);
    assert_eq!(context.expected_type, SemanticType::I64);
    assert_eq!(context.owner_function, range);
    assert_eq!(context.function_signature.parameter_count, 1);
    assert_eq!(context.function_signature.result, SemanticType::I64);
    assert_eq!(context.owner_chain.first().expect("hole owner").node, hole);
    assert!(
        context
            .owner_chain
            .iter()
            .any(|fact| fact.node == context.owner_block)
    );
    assert!(context.owner_chain.iter().any(|fact| fact.node == range));
    let for_region = context
        .enclosing_regions
        .iter()
        .find(|fact| fact.role == RegionRole::ForBody)
        .expect("enclosing for body");
    assert_eq!(for_region.region, context.visible_block_arguments[0].region);
    let hole_position = context
        .body_window
        .iter()
        .position(|item| item.operation == hole && item.code == OperationCode::Hole)
        .expect("hole body item");
    assert_eq!(
        context.body_window[hole_position + 1].code,
        OperationCode::Yield
    );
    assert!(context.visible_values.items.iter().any(|value| {
        matches!(value.value, ValueRef::FunctionParameter { .. })
            && value.owner_function == range
            && value.ty == SemanticType::I64
    }));
    assert!(context.visible_values.items.iter().any(|value| {
        value.producer_code == Some(OperationCode::ConstI64)
            && value.owner_function == range
            && value.ordinal == Some(0)
    }));
    assert_eq!(
        context.blocker.as_ref().and_then(|blocker| blocker.target),
        Some(hole)
    );
    let add_constructor = context
        .legal_constructors
        .iter()
        .find(|constructor| constructor.code == OperationCode::AddI64)
        .expect("direct add_i64 constructor");
    assert!(add_constructor.direct_refinement);
    assert_eq!(
        add_constructor.operand_types,
        vec![SemanticType::I64, SemanticType::I64]
    );
    let index = context
        .visible_block_arguments
        .iter()
        .find(|fact| fact.role == BlockArgumentRole::LoopIndex)
        .expect("loop index fact");
    let carried = context
        .visible_block_arguments
        .iter()
        .find(|fact| fact.role == BlockArgumentRole::LoopCarried)
        .expect("loop carried fact");
    assert_eq!(index.ordinal, 0);
    assert_eq!(carried.ordinal, 1);
    assert_eq!(index.ty, SemanticType::I64);
    assert_eq!(carried.ty, SemanticType::I64);
    assert_eq!(index.block, carried.block);
    assert_eq!(index.region, carried.region);
    let yield_use = context
        .incoming_uses
        .items
        .iter()
        .find(|site| site.operand_index == 0)
        .expect("yield use")
        .source;

    let head = workspace_path(state, workspace).join("HEAD");
    let head_before_invalid = fs::read(&head).expect("HEAD before invalid refinement");
    let invalid = ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::ConstBool(true),
            }],
        },
        response: TransactionResponseSpec::default(),
    };
    let Response::Error(invalid_error) =
        rpc(state, 503, Request::ApplyTransaction(invalid)).response
    else {
        panic!("invalid refinement")
    };
    assert!(matches!(
        invalid_error.code,
        ErrorCode::InvalidOperand | ErrorCode::TypeMismatch
    ));
    assert_eq!(
        fs::read(&head).expect("HEAD after invalid refinement"),
        head_before_invalid
    );
    assert_eq!(revision_files(state, workspace), revision_one_files);

    // Context returns persistent IDs, so refinement uses Existing targets rather than draft handles.
    let valid = ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::AddI64 {
                    lhs: ValueDraft::BlockArgument(NodeTarget::Existing(carried.argument)),
                    rhs: ValueDraft::BlockArgument(NodeTarget::Existing(index.argument)),
                },
            }],
        },
        response: TransactionResponseSpec::default(),
    };
    let refined = receipt(rpc(state, 504, Request::ApplyTransaction(valid)));
    assert_eq!(refined.created_count, 0);
    assert!(refined.complete_after);
    let diff = collect_diff(state, workspace, Revision::new(1), Revision::new(2));
    assert!(diff.2.iter().any(|change| change.node == hole
        && matches!(
            change.kind,
            ChangeKind::OperationRefined {
                before: OperationCode::Hole,
                after: OperationCode::AddI64,
                ..
            }
        )));
    let QueryResult::IncomingUses(uses) = query(
        state,
        505,
        workspace,
        Revision::new(2),
        Query::IncomingUses {
            value: ValueRef::OperationResult {
                operation: hole,
                output: 0,
            },
            page: PageRequest {
                after: None,
                limit: 8,
            },
        },
    ) else {
        panic!("uses")
    };
    assert!(
        uses.items
            .iter()
            .any(|site| site.source == yield_use && site.operand_index == 0)
    );

    let run = |id, revision, entry, arguments| {
        rpc(
            state,
            id,
            Request::Run {
                workspace,
                revision,
                entry,
                arguments,
                policy: lkjscript::RunPolicy {
                    fuel: 1_000_000,
                    maximum_frames: 10_000,
                },
            },
        )
    };
    let Response::Error(incomplete) = run(506, Revision::new(1), main, vec![]).response else {
        panic!("incomplete run")
    };
    assert_eq!(incomplete.code, ErrorCode::CompileIncomplete);
    for (id, entry, arguments, expected) in [
        (507, main, vec![], 5050),
        (508, normalize, vec![RuntimeValue::I64(-3)], 0),
        (509, normalize, vec![RuntimeValue::I64(11)], 55),
    ] {
        let Response::Run(result) = run(id, Revision::new(2), entry, arguments).response else {
            panic!("run result")
        };
        assert_eq!(result.value, RuntimeValue::I64(expected));
    }

    shutdown(state, 510);
    daemon.wait();
    let daemon = JsonDaemon::start(state);
    for revision in [Revision::new(1), Revision::new(2)] {
        for (offset, node) in [hole, range, normalize, main].into_iter().enumerate() {
            let QueryResult::Node(view) = query(
                state,
                520 + revision.get() * 10 + offset as u64,
                workspace,
                revision,
                Query::Node {
                    node,
                    expand: false,
                },
            ) else {
                panic!("retained node")
            };
            assert_eq!(view.summary.node, node);
            assert_eq!(view.summary.revision, revision);
        }
    }
    let Response::Error(incomplete) = run(550, Revision::new(1), main, vec![]).response else {
        panic!("retained incomplete run")
    };
    assert_eq!(incomplete.code, ErrorCode::CompileIncomplete);
    let Response::Run(repaired) = run(551, Revision::new(2), main, vec![]).response else {
        panic!("retained repaired run")
    };
    assert_eq!(repaired.value, RuntimeValue::I64(5050));
    shutdown(state, 552);
    daemon.wait();
}

#[test]
#[ignore = "manual structured generic-CLI interaction-cost measurement"]
fn structured_agent_interaction_cost_measurement() {
    let temporary = tempfile::tempdir().expect("state directory");
    let state = temporary.path();
    let cold_started = Instant::now();
    let daemon = JsonDaemon::start(state);
    let cold_start_ns = cold_started.elapsed().as_nanos();
    let (_, schema) = measured_rpc(state, 600, "schema_discovery", Request::DescribeSchema);
    let (workspace_response, workspace_metric) =
        measured_rpc(state, 601, "workspace_create", Request::CreateWorkspace);
    let Response::WorkspaceCreated(initial) = workspace_response.response else {
        panic!("workspace")
    };
    let workspace = initial.workspace;
    let creation_request = structured_creation(workspace);
    let transaction_items = creation_request.transaction.operations.len();
    let requested_bindings = creation_request.response.return_handles.len();
    let (creation_response, creation_metric) = measured_rpc(
        state,
        602,
        "structured_creation",
        Request::ApplyTransaction(creation_request),
    );
    let Response::TransactionReceipt(created) = creation_response.response else {
        panic!("creation")
    };
    let hole = binding(&created, 16);
    let normalize = binding(&created, 20);
    let main = binding(&created, 30);
    let incomplete_artifact_bytes = fs::metadata(
        workspace_path(state, workspace).join("revisions/00000000000000000001.lkjscript"),
    )
    .expect("incomplete artifact")
    .len();

    let context_request = Request::QueryBatch(QueryBatchRequest {
        workspace,
        revision: Revision::new(1),
        queries: vec![QueryItem {
            id: QueryId::new(1),
            query: Query::RepairContext {
                target: RepairTarget::Hole(hole),
                budget: context_budget(),
            },
        }],
    });
    let (context_response, context_metric) =
        measured_rpc(state, 603, "repair_context", context_request);
    let Response::QueryBatchResult(batch) = context_response.response else {
        panic!("context batch")
    };
    let QueryOutcome::Success(context) = &batch.results[0].outcome else {
        panic!("context outcome")
    };
    let QueryResult::RepairContext(context) = context.as_ref() else {
        panic!("context")
    };
    let index = context
        .visible_block_arguments
        .iter()
        .find(|fact| fact.role == BlockArgumentRole::LoopIndex)
        .expect("index")
        .argument;
    let carried = context
        .visible_block_arguments
        .iter()
        .find(|fact| fact.role == BlockArgumentRole::LoopCarried)
        .expect("carried")
        .argument;

    let invalid_request = Request::ApplyTransaction(ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::ConstBool(true),
            }],
        },
        response: TransactionResponseSpec::default(),
    });
    let (invalid_response, invalid_metric) =
        measured_rpc(state, 604, "invalid_repair", invalid_request);
    assert!(matches!(invalid_response.response, Response::Error(_)));
    let repair_request = Request::ApplyTransaction(ApplyTransactionRequest {
        transaction: Transaction {
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
        },
        response: TransactionResponseSpec::default(),
    });
    let (repair_response, repair_metric) = measured_rpc(state, 605, "valid_repair", repair_request);
    let Response::TransactionReceipt(repaired) = repair_response.response else {
        panic!("repair")
    };
    assert_eq!(repaired.created_count, 0);
    let repaired_artifact_bytes = fs::metadata(
        workspace_path(state, workspace).join("revisions/00000000000000000002.lkjscript"),
    )
    .expect("repaired artifact")
    .len();

    let diff_request = Request::QueryBatch(QueryBatchRequest {
        workspace,
        revision: Revision::new(2),
        queries: vec![QueryItem {
            id: QueryId::new(2),
            query: Query::SemanticDiff {
                from: Revision::new(1),
                page: PageRequest {
                    after: None,
                    limit: 64,
                },
            },
        }],
    });
    let (_, diff_metric) = measured_rpc(state, 606, "semantic_diff", diff_request);
    let mut run_metrics = Vec::new();
    for (request_id, name, entry, arguments, expected) in [
        (607, "run_main", main, vec![], 5050),
        (
            608,
            "run_negative",
            normalize,
            vec![RuntimeValue::I64(-3)],
            0,
        ),
        (
            609,
            "run_eleven",
            normalize,
            vec![RuntimeValue::I64(11)],
            55,
        ),
    ] {
        let request = Request::Run {
            workspace,
            revision: Revision::new(2),
            entry,
            arguments,
            policy: lkjscript::RunPolicy {
                fuel: 1_000_000,
                maximum_frames: 10_000,
            },
        };
        let (response, metric) = measured_rpc(state, request_id, name, request);
        let Response::Run(result) = response.response else {
            panic!("run")
        };
        assert_eq!(result.value, RuntimeValue::I64(expected));
        run_metrics.push(metric);
    }
    shutdown(state, 610);
    daemon.wait();
    let restart_started = Instant::now();
    let daemon = JsonDaemon::start(state);
    let restart_ns = restart_started.elapsed().as_nanos();
    let restart_request = Request::QueryBatch(QueryBatchRequest {
        workspace,
        revision: Revision::new(2),
        queries: vec![QueryItem {
            id: QueryId::new(3),
            query: Query::Node {
                node: hole,
                expand: false,
            },
        }],
    });
    let (_, restart_query_metric) =
        measured_rpc(state, 611, "restart_retained_query", restart_request);

    let all_metrics = [
        &schema,
        &workspace_metric,
        &creation_metric,
        &context_metric,
        &invalid_metric,
        &repair_metric,
        &diff_metric,
        &run_metrics[0],
        &run_metrics[1],
        &run_metrics[2],
        &restart_query_metric,
    ];
    println!(
        "STRUCTURED_AGENT_COST {}",
        serde_json::json!({
            "round_trips": all_metrics.len(),
            "cli_invocations": all_metrics.len(),
            "daemon_cold_start_ns": cold_start_ns,
            "restart_ns": restart_ns,
            "public_transaction_items": transaction_items,
            "explicit_requested_bindings": requested_bindings,
            "canonical_created_nodes": created.created_count,
            "incomplete_artifact_bytes": incomplete_artifact_bytes,
            "repaired_artifact_bytes": repaired_artifact_bytes,
            "json_request_bytes": all_metrics.iter().map(|metric| metric.json_request_bytes).sum::<usize>(),
            "json_stdout_bytes": all_metrics.iter().map(|metric| metric.json_stdout_bytes).sum::<usize>(),
            "binary_request_bytes": all_metrics.iter().map(|metric| metric.binary_request_bytes).sum::<usize>(),
            "binary_response_bytes": all_metrics.iter().map(|metric| metric.binary_response_bytes).sum::<usize>(),
            "cli_wall_ns": all_metrics.iter().map(|metric| metric.elapsed_ns).sum::<u128>(),
            "per_request": all_metrics.iter().map(|metric| metric_json(metric)).collect::<Vec<_>>(),
            "oracles": [5050, 0, 55],
            "model_tokens_measured": false,
        })
    );
    shutdown(state, 612);
    daemon.wait();
}

fn percentile_ns(samples: &[u128], percentile: usize) -> u128 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = (sorted.len() * percentile).div_ceil(100).saturating_sub(1);
    sorted[rank.min(sorted.len() - 1)]
}

fn timed_rpc(state: &Path, request_id: u64, request: Request) -> (ResponseEnvelope, u128) {
    let started = Instant::now();
    let response = rpc(state, request_id, request);
    (response, started.elapsed().as_nanos())
}

#[test]
#[ignore = "manual repeated structured product-path performance measurement"]
fn structured_product_path_performance_measurement() {
    let mut cold_start = Vec::new();
    for sample in 0..12 {
        let temporary = tempfile::tempdir().expect("cold state");
        let started = Instant::now();
        let daemon = JsonDaemon::start(temporary.path());
        let elapsed = started.elapsed().as_nanos();
        shutdown(temporary.path(), 7000 + sample);
        daemon.wait();
        if sample > 0 {
            cold_start.push(elapsed);
        }
    }

    let temporary = tempfile::tempdir().expect("performance state");
    let state = temporary.path();
    let daemon = JsonDaemon::start(state);
    let mut workspace_create = Vec::new();
    let mut workspaces = Vec::new();
    for sample in 0..32 {
        let (response, elapsed) = timed_rpc(state, 7100 + sample, Request::CreateWorkspace);
        let Response::WorkspaceCreated(summary) = response.response else {
            panic!("workspace")
        };
        if sample > 0 {
            workspace_create.push(elapsed);
        }
        workspaces.push(summary.workspace);
    }
    let mut structured_commit = Vec::new();
    let mut selected = None;
    for (sample, workspace) in workspaces.into_iter().take(12).enumerate() {
        let request = Request::ApplyTransaction(structured_creation(workspace));
        let (response, elapsed) = timed_rpc(state, 7200 + sample as u64, request);
        let Response::TransactionReceipt(receipt) = response.response else {
            panic!("commit")
        };
        if sample > 0 {
            structured_commit.push(elapsed);
        }
        selected = Some((workspace, receipt));
    }
    let (workspace, created) = selected.expect("selected workspace");
    let hole = binding(&created, 16);
    let normalize = binding(&created, 20);
    let main = binding(&created, 30);
    let context_request = || {
        Request::QueryBatch(QueryBatchRequest {
            workspace,
            revision: Revision::new(1),
            queries: vec![QueryItem {
                id: QueryId::new(1),
                query: Query::RepairContext {
                    target: RepairTarget::Hole(hole),
                    budget: context_budget(),
                },
            }],
        })
    };
    let mut repair_context = Vec::new();
    let mut context_value = None;
    for sample in 0..32 {
        let (response, elapsed) = timed_rpc(state, 7300 + sample, context_request());
        let Response::QueryBatchResult(batch) = response.response else {
            panic!("context")
        };
        let QueryOutcome::Success(result) = &batch.results[0].outcome else {
            panic!("context outcome")
        };
        let QueryResult::RepairContext(context) = result.as_ref() else {
            panic!("context result")
        };
        context_value = Some(context.clone());
        if sample > 0 {
            repair_context.push(elapsed);
        }
    }
    let context = context_value.expect("context value");
    let index = context
        .visible_block_arguments
        .iter()
        .find(|fact| fact.role == BlockArgumentRole::LoopIndex)
        .expect("index")
        .argument;
    let carried = context
        .visible_block_arguments
        .iter()
        .find(|fact| fact.role == BlockArgumentRole::LoopCarried)
        .expect("carried")
        .argument;
    let repair = Request::ApplyTransaction(ApplyTransactionRequest {
        transaction: Transaction {
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
        },
        response: TransactionResponseSpec::default(),
    });
    let Response::TransactionReceipt(_) = rpc(state, 7400, repair).response else {
        panic!("repair")
    };

    let run_request = |entry, arguments, fuel| Request::Run {
        workspace,
        revision: Revision::new(2),
        entry,
        arguments,
        policy: lkjscript::RunPolicy {
            fuel,
            maximum_frames: 10_000,
        },
    };
    let mut main_wall = Vec::new();
    let mut main_compile = Vec::new();
    let mut main_execute = Vec::new();
    let mut direct_wall = Vec::new();
    let mut recursion_wall = Vec::new();
    let mut fuel_wall = Vec::new();
    for sample in 0..32 {
        let (response, elapsed) =
            timed_rpc(state, 7500 + sample, run_request(main, vec![], 1_000_000));
        let Response::Run(result) = response.response else {
            panic!("main run")
        };
        assert_eq!(result.value, RuntimeValue::I64(5050));
        if sample > 0 {
            main_wall.push(elapsed);
            main_compile.push(u128::from(result.compile_nanoseconds));
            main_execute.push(u128::from(result.execute_nanoseconds));
        }
        let (response, elapsed) = timed_rpc(
            state,
            7600 + sample,
            run_request(normalize, vec![RuntimeValue::I64(11)], 1_000_000),
        );
        let Response::Run(result) = response.response else {
            panic!("direct run")
        };
        assert_eq!(result.value, RuntimeValue::I64(55));
        if sample > 0 {
            direct_wall.push(elapsed);
        }
        let (response, elapsed) = timed_rpc(state, 7700 + sample, run_request(main, vec![], 1));
        assert!(
            matches!(response.response, Response::Error(ref error) if error.code == ErrorCode::ExecutionFuelExhausted)
        );
        if sample > 0 {
            fuel_wall.push(elapsed);
        }
    }

    let recursion_workspace = match rpc(state, 7800, Request::CreateWorkspace).response {
        Response::WorkspaceCreated(summary) => summary.workspace,
        _ => panic!("recursion workspace"),
    };
    let local = |handle| NodeTarget::Local(LocalHandle::new(handle));
    let parameter = ValueDraft::FunctionParameter(local(4));
    let recursion_creation = ApplyTransactionRequest {
        transaction: Transaction {
            workspace: recursion_workspace,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "recursion".into(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(3),
                    module: local(2),
                    name: "once".into(),
                    parameters: vec![FunctionParameterDraft {
                        handle: LocalHandle::new(4),
                        name: "again".into(),
                        ty: SemanticType::Bool,
                    }],
                    result: SemanticType::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![ExpressionDraft {
                            handle: LocalHandle::new(5),
                            operation: ExpressionKindDraft::If {
                                condition: parameter,
                                result: SemanticType::I64,
                                then_body: YieldingBodyDraft {
                                    operations: vec![
                                        ExpressionDraft {
                                            handle: LocalHandle::new(6),
                                            operation: ExpressionKindDraft::ConstBool(false),
                                        },
                                        ExpressionDraft {
                                            handle: LocalHandle::new(7),
                                            operation: ExpressionKindDraft::Call {
                                                function: local(3),
                                                arguments: vec![ValueDraft::OperationResult {
                                                    operation: local(6),
                                                    output: 0,
                                                }],
                                            },
                                        },
                                    ],
                                    yield_value: ValueDraft::OperationResult {
                                        operation: local(7),
                                        output: 0,
                                    },
                                },
                                else_body: YieldingBodyDraft {
                                    operations: vec![ExpressionDraft {
                                        handle: LocalHandle::new(8),
                                        operation: ExpressionKindDraft::ConstI64(1),
                                    }],
                                    yield_value: ValueDraft::OperationResult {
                                        operation: local(8),
                                        output: 0,
                                    },
                                },
                            },
                        }],
                        return_value: ValueDraft::OperationResult {
                            operation: local(5),
                            output: 0,
                        },
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: local(1),
                    function: local(3),
                },
            ],
        },
        response: TransactionResponseSpec {
            return_handles: vec![LocalHandle::new(3)],
        },
    };
    let recursion_receipt = receipt(rpc(
        state,
        7801,
        Request::ApplyTransaction(recursion_creation),
    ));
    let recursion_entry = binding(&recursion_receipt, 3);
    for sample in 0..32 {
        let request = Request::Run {
            workspace: recursion_workspace,
            revision: Revision::new(1),
            entry: recursion_entry,
            arguments: vec![RuntimeValue::Bool(true)],
            policy: lkjscript::RunPolicy {
                fuel: 100,
                maximum_frames: 10,
            },
        };
        let (response, elapsed) = timed_rpc(state, 7900 + sample, request);
        assert!(
            matches!(response.response, Response::Run(ref result) if result.value == RuntimeValue::I64(1))
        );
        if sample > 0 {
            recursion_wall.push(elapsed);
        }
    }

    shutdown(state, 8000);
    daemon.wait();
    let mut restart = Vec::new();
    for sample in 0..12 {
        let started = Instant::now();
        let daemon = JsonDaemon::start(state);
        let elapsed = started.elapsed().as_nanos();
        shutdown(state, 8010 + sample);
        daemon.wait();
        if sample > 0 {
            restart.push(elapsed);
        }
    }
    let row = |samples: &[u128]| serde_json::json!({"samples": samples.len(), "median_ns": percentile_ns(samples, 50), "p95_ns": percentile_ns(samples, 95)});
    println!(
        "STRUCTURED_PRODUCT_PATH {}",
        serde_json::json!({
            "warmup_samples": 1,
            "cold_start": row(&cold_start),
            "workspace_create": row(&workspace_create),
            "structured_commit": row(&structured_commit),
            "repair_context": row(&repair_context),
            "main_request_wall": row(&main_wall),
            "main_compile": row(&main_compile),
            "main_execute": row(&main_execute),
            "direct_parameterized_run": row(&direct_wall),
            "finite_recursion": row(&recursion_wall),
            "controlled_fuel_exhaustion": row(&fuel_wall),
            "restart_retained_workspaces": row(&restart),
            "oracles": {"main": 5050, "direct": 55, "recursion": 1, "fuel_error": "execution_fuel_exhausted"},
        })
    );
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
        arguments: vec![],
        policy: lkjscript::RunPolicy {
            fuel: 1_000_000,
            maximum_frames: 10_000,
        },
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
