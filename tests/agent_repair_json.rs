#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use lkjscript::diff::ChangeKind;
use lkjscript::machine::{
    BoundaryErrorEnvelope, BoundaryErrorKind, DescribeSchemaRequest, DescribeSchemaResult,
    JSON_ENVELOPE_VERSION, RequestEnvelope, ResponseEnvelope, SchemaProjection, SchemaRoot,
};
use lkjscript::query::{
    CompletenessBlocker, ContextBudget, Page, PageCursor, PageRequest, Query, QueryBatchRequest,
    QueryItem, QueryOutcome, QueryResult, RepairContext, RepairTarget,
};
use lkjscript::{
    ApplyTransactionRequest, BlockArgumentRole, ChangeDigest, DraftSymbol, ErrorCode,
    ExpressionDraft, ExpressionKindDraft, FunctionBodyDraft, FunctionParameterDraft,
    IdempotencyKey, MatchArmDraft, NodeId, NodeKind, NodeTarget, OperationCode, OperationDraft,
    OperationKind, ProductFieldDraft, ProductFieldValueDraft, QueryId, RegionRole, Request,
    RequestId, Response, Revision, RuntimeFieldValue, RuntimeValue, SemanticType, SumVariantDraft,
    Transaction, TransactionMode, TransactionOp, TransactionReceipt, TransactionResponseSpec,
    TypeDraft, ValueDraft, ValueRef, WorkspaceId, YieldingBodyDraft,
};
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

#[test]
fn real_json_cli_repairs_hole_and_operand_across_direct_reopen() {
    let temporary = tempfile::tempdir().expect("state directory");
    let state = temporary.path();

    let raw_create = invoke_raw(
        state,
        br#"{"version":9,"request_id":1,"request":{"kind":"create_workspace"}}"#,
    );
    assert!(raw_create.status.success());
    assert!(raw_create.stderr.is_empty());
    let created: ResponseEnvelope =
        serde_json::from_slice(&raw_create.stdout).expect("raw create response");
    assert_eq!(created.request_id, RequestId::new(1));
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
    assert_eq!(committed.created_count, 4);
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
        panic!("zero-page engine error")
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
        panic!("duplicate query engine error")
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
            OperationCode::BytesLen,
            OperationCode::BytesAt,
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
            OperationCode::BytesLen,
            OperationCode::BytesAt,
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
                        symbol: Some(DraftSymbol::new("s100")),
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
            return_symbols: vec![DraftSymbol::new("s100")],
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
    assert_eq!(bulk.created_count, 0);
    assert_eq!(bulk.returned_bindings.len(), 1);
    assert_eq!(binding(&bulk, 100), predicted_frontier);
    assert!(predicted_frontier.is_function_local());
    let mut all_bindings = bulk.clone();
    let first_local_ordinal = predicted_frontier.local_ordinal().expect("local frontier");
    all_bindings.returned_bindings = (0..200_u32)
        .map(|offset| {
            (
                DraftSymbol::new(&format!("s{}", 100 + offset)),
                NodeId::new_function_local(workspace, function, first_local_ordinal + offset)
                    .expect("bulk predicted local reference"),
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

    let Response::DescribeSchema(engine_schema) = rpc(
        state,
        37,
        Request::DescribeSchema(DescribeSchemaRequest::manifest()),
    )
    .response
    else {
        panic!("engine schema")
    };
    let local_schema = local_schema();
    assert_eq!(*engine_schema, local_schema);

    let root_request = DescribeSchemaRequest {
        projection: SchemaProjection::Roots {
            roots: vec![SchemaRoot::Query, SchemaRoot::TransactionOperation],
        },
        known_digest: None,
    };
    let Response::DescribeSchema(engine_roots) =
        rpc(state, 38, Request::DescribeSchema(root_request)).response
    else {
        panic!("engine schema roots")
    };
    let local_roots = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args([
            "schema",
            "--root",
            "query",
            "--root",
            "transaction_operation",
        ])
        .output()
        .expect("local schema roots");
    assert!(local_roots.status.success());
    assert_eq!(
        *engine_roots,
        serde_json::from_slice(&local_roots.stdout).expect("local root JSON")
    );
    assert_eq!(module.workspace(), workspace);

    let zero_request_id = invoke_raw(
        state,
        br#"{"version":9,"request_id":0,"request":{"kind":"shutdown"}}"#,
    );
    assert_eq!(zero_request_id.status.code(), Some(2));
    let zero_request_id: BoundaryErrorEnvelope =
        serde_json::from_slice(&zero_request_id.stdout).expect("zero request ID boundary JSON");
    assert_eq!(zero_request_id.request_id, None);
    assert_eq!(zero_request_id.error.kind, BoundaryErrorKind::InvalidJson);

    let malformed = invoke_raw(
        state,
        br#"{"version":9,"request_id":99,"request":{"kind":"shutdown","unknown":true}}"#,
    );
    assert_eq!(malformed.status.code(), Some(2));
    assert_one_json(&malformed.stdout);
    let malformed: BoundaryErrorEnvelope =
        serde_json::from_slice(&malformed.stdout).expect("malformed boundary JSON");
    assert_eq!(malformed.request_id, Some(RequestId::new(99)));
    assert_eq!(malformed.error.kind, BoundaryErrorKind::InvalidJson);

    shutdown(state, 40);
}

fn incomplete_fixture(
    workspace: WorkspaceId,
    mode: TransactionMode,
    idempotency_key: Option<IdempotencyKey>,
) -> ApplyTransactionRequest {
    let local = NodeTarget::Draft;
    let result = |symbol| ValueDraft::OperationResult {
        operation: local(symbol),
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
                    symbol: DraftSymbol::new("s1"),
                    name: "app".to_owned(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::new("s2"),
                    package: local(DraftSymbol::new("s1")),
                    name: "root".to_owned(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::new("s3"),
                    module: local(DraftSymbol::new("s2")),
                    name: "main".to_owned(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::new("s6")),
                                operation: ExpressionKindDraft::ConstI64(40),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::new("s7")),
                                operation: ExpressionKindDraft::ConstI64(2),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::new("s8")),
                                operation: ExpressionKindDraft::ConstBool(true),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::new("s9")),
                                operation: ExpressionKindDraft::Hole {
                                    expected: SemanticType::I64.into(),
                                },
                            },
                        ],
                        return_value: result(DraftSymbol::new("s9")),
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: local(DraftSymbol::new("s1")),
                    function: local(DraftSymbol::new("s3")),
                },
            ],
        },
        response: TransactionResponseSpec {
            return_symbols: [2, 3, 6, 7, 8, 9]
                .into_iter()
                .map(|value| DraftSymbol::new(&format!("s{value}")))
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
                    symbol: Some(DraftSymbol::new("s100")),
                    operation: ExpressionKindDraft::ConstI64(7),
                },
            }],
        },
        response: TransactionResponseSpec {
            return_symbols: vec![DraftSymbol::new("s100")],
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
                        symbol: Some(DraftSymbol::new(&format!("s{}", 100 + index))),
                        operation: ExpressionKindDraft::ConstI64(i64::from(index)),
                    },
                })
                .collect(),
        },
        response: TransactionResponseSpec {
            return_symbols: vec![DraftSymbol::new("s100")],
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
    one_query_result(
        rpc(
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
        ),
        workspace,
        revision,
        QueryId::new(request_id),
    )
}

fn one_query_result(
    response: ResponseEnvelope,
    workspace: WorkspaceId,
    revision: Revision,
    query_id: QueryId,
) -> QueryResult {
    let Response::QueryBatchResult(batch) = response.response else {
        panic!("query batch response")
    };
    assert_eq!(batch.workspace, workspace);
    assert_eq!(batch.revision, revision);
    assert_eq!(batch.results.len(), 1);
    let item = batch.results.into_iter().next().expect("query item");
    assert_eq!(item.id, query_id);
    let QueryOutcome::Success(result) = item.outcome else {
        panic!("query outcome must be semantic success")
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

fn binding(receipt: &TransactionReceipt, symbol: u32) -> NodeId {
    receipt
        .returned_bindings
        .iter()
        .find_map(|(candidate, node)| {
            (*candidate == DraftSymbol::new(&format!("s{symbol}"))).then_some(*node)
        })
        .expect("selected binding")
}

fn structured_creation(workspace: WorkspaceId) -> ApplyTransactionRequest {
    let local = |symbol| NodeTarget::Draft(DraftSymbol::new(&format!("s{symbol}")));
    let result = |symbol| ValueDraft::OperationResult {
        operation: local(symbol),
        output: 0,
    };
    let parameter = |symbol| ValueDraft::FunctionParameter(local(symbol));
    ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::new("s1"),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::new("s2"),
                    package: local(1),
                    name: "structured".into(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::new("s10"),
                    module: local(2),
                    name: "range_sum".into(),
                    parameters: vec![FunctionParameterDraft {
                        symbol: DraftSymbol::new("s11"),
                        name: "n".into(),
                        ty: SemanticType::I64.into(),
                    }],
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::new("s12")),
                                operation: ExpressionKindDraft::ConstI64(0),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::new("s13")),
                                operation: ExpressionKindDraft::ForI64 {
                                    start: result(12),
                                    end_exclusive: parameter(11),
                                    step: 1,
                                    initial: result(12),
                                    carried: SemanticType::I64.into(),
                                    index_symbol: DraftSymbol::new("s14"),
                                    carried_symbol: DraftSymbol::new("s15"),
                                    body: YieldingBodyDraft {
                                        operations: vec![ExpressionDraft {
                                            symbol: Some(DraftSymbol::new("s16")),
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
                    symbol: DraftSymbol::new("s20"),
                    module: local(2),
                    name: "normalize_and_sum".into(),
                    parameters: vec![FunctionParameterDraft {
                        symbol: DraftSymbol::new("s21"),
                        name: "n".into(),
                        ty: SemanticType::I64.into(),
                    }],
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::new("s22")),
                                operation: ExpressionKindDraft::ConstI64(0),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::new("s23")),
                                operation: ExpressionKindDraft::LtI64 {
                                    lhs: parameter(21),
                                    rhs: result(22),
                                },
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::new("s24")),
                                operation: ExpressionKindDraft::If {
                                    condition: result(23),
                                    result: SemanticType::I64.into(),
                                    then_body: YieldingBodyDraft {
                                        operations: vec![],
                                        yield_value: result(22),
                                    },
                                    else_body: YieldingBodyDraft {
                                        operations: vec![ExpressionDraft {
                                            symbol: Some(DraftSymbol::new("s25")),
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
                    symbol: DraftSymbol::new("s30"),
                    module: local(2),
                    name: "main".into(),
                    parameters: vec![],
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::new("s31")),
                                operation: ExpressionKindDraft::ConstI64(101),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::new("s32")),
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
            return_symbols: vec![
                DraftSymbol::new("s10"),
                DraftSymbol::new("s16"),
                DraftSymbol::new("s20"),
                DraftSymbol::new("s30"),
            ],
        },
    }
}

#[test]
fn real_json_cli_structured_program_repair_vertical() {
    let temporary = tempfile::tempdir().expect("state directory");
    let state = temporary.path();
    let Response::WorkspaceCreated(initial) = rpc(state, 500, Request::CreateWorkspace).response
    else {
        panic!("workspace")
    };
    let workspace = initial.workspace;
    let creation = structured_creation(workspace);
    assert_eq!(creation.transaction.operations.len(), 6);
    let created = receipt(rpc(state, 501, Request::ApplyTransaction(creation)));
    assert_eq!(created.returned_bindings.len(), 4);
    assert_eq!(created.created_count, 8);
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
}

fn nominal_reading_application(workspace: WorkspaceId) -> ApplyTransactionRequest {
    let local = |symbol| NodeTarget::Draft(DraftSymbol::new(&format!("s{symbol}")));
    let result = |symbol| ValueDraft::OperationResult {
        operation: local(symbol),
        output: 0,
    };
    let parameter = |symbol| ValueDraft::FunctionParameter(local(symbol));
    let payload = |symbol| ValueDraft::BlockArgument(local(symbol));
    let expression = |symbol, operation| ExpressionDraft {
        symbol: Some(DraftSymbol::new(&format!("s{symbol}"))),
        operation,
    };
    let field = |symbol, value| ProductFieldValueDraft {
        field: local(symbol),
        value,
    };
    ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::INITIAL,
            idempotency_key: Some(IdempotencyKey::from_bytes([0x81; 16])),
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::new("s1"),
                    name: "reading-app".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::new("s2"),
                    package: local(1),
                    name: "root".into(),
                },
                // Functions deliberately precede their local nominal declarations and members.
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::new("s10"),
                    module: local(2),
                    name: "evaluate".into(),
                    parameters: vec![FunctionParameterDraft {
                        symbol: DraftSymbol::new("s11"),
                        name: "input".into(),
                        ty: TypeDraft::Nominal(local(6)),
                    }],
                    result: TypeDraft::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![expression(
                            12,
                            ExpressionKindDraft::MatchSum {
                                scrutinee: parameter(11),
                                result: TypeDraft::I64,
                                arms: vec![
                                    MatchArmDraft {
                                        variant: local(9),
                                        payload_symbol: Some(DraftSymbol::new("s19")),
                                        body: YieldingBodyDraft {
                                            operations: vec![],
                                            yield_value: payload(19),
                                        },
                                    },
                                    MatchArmDraft {
                                        variant: local(7),
                                        payload_symbol: Some(DraftSymbol::new("s13")),
                                        body: YieldingBodyDraft {
                                            operations: vec![
                                                expression(
                                                    14,
                                                    ExpressionKindDraft::ProjectField {
                                                        value: payload(13),
                                                        field: local(4),
                                                    },
                                                ),
                                                expression(
                                                    15,
                                                    ExpressionKindDraft::ProjectField {
                                                        value: payload(13),
                                                        field: local(5),
                                                    },
                                                ),
                                                expression(16, ExpressionKindDraft::ConstI64(0)),
                                                expression(
                                                    17,
                                                    ExpressionKindDraft::If {
                                                        condition: result(15),
                                                        result: TypeDraft::I64,
                                                        then_body: YieldingBodyDraft {
                                                            operations: vec![],
                                                            yield_value: result(14),
                                                        },
                                                        else_body: YieldingBodyDraft {
                                                            operations: vec![],
                                                            yield_value: result(16),
                                                        },
                                                    },
                                                ),
                                            ],
                                            yield_value: result(17),
                                        },
                                    },
                                    MatchArmDraft {
                                        variant: local(8),
                                        payload_symbol: None,
                                        body: YieldingBodyDraft {
                                            operations: vec![expression(
                                                18,
                                                ExpressionKindDraft::ConstI64(0),
                                            )],
                                            yield_value: result(18),
                                        },
                                    },
                                ],
                            },
                        )],
                        return_value: result(12),
                    }),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::new("s30"),
                    module: local(2),
                    name: "main".into(),
                    parameters: vec![],
                    result: TypeDraft::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            expression(31, ExpressionKindDraft::ConstI64(42)),
                            expression(32, ExpressionKindDraft::ConstBool(true)),
                            expression(
                                33,
                                ExpressionKindDraft::Hole {
                                    expected: TypeDraft::Nominal(local(3)),
                                },
                            ),
                            expression(
                                34,
                                ExpressionKindDraft::ConstructVariant {
                                    variant: local(7),
                                    payload: Some(result(33)),
                                },
                            ),
                            expression(
                                35,
                                ExpressionKindDraft::Call {
                                    function: local(10),
                                    arguments: vec![result(34)],
                                },
                            ),
                        ],
                        return_value: result(35),
                    }),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::new("s40"),
                    module: local(2),
                    name: "evaluate_disabled".into(),
                    parameters: vec![FunctionParameterDraft {
                        symbol: DraftSymbol::new("s41"),
                        name: "value".into(),
                        ty: TypeDraft::I64,
                    }],
                    result: TypeDraft::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            expression(42, ExpressionKindDraft::ConstBool(false)),
                            expression(
                                43,
                                ExpressionKindDraft::ConstructProduct {
                                    product: local(3),
                                    fields: vec![field(5, result(42)), field(4, parameter(41))],
                                },
                            ),
                            expression(
                                44,
                                ExpressionKindDraft::ConstructVariant {
                                    variant: local(7),
                                    payload: Some(result(43)),
                                },
                            ),
                            expression(
                                45,
                                ExpressionKindDraft::Call {
                                    function: local(10),
                                    arguments: vec![result(44)],
                                },
                            ),
                        ],
                        return_value: result(45),
                    }),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::new("s50"),
                    module: local(2),
                    name: "evaluate_missing".into(),
                    parameters: vec![],
                    result: TypeDraft::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            expression(
                                51,
                                ExpressionKindDraft::ConstructVariant {
                                    variant: local(8),
                                    payload: None,
                                },
                            ),
                            expression(
                                52,
                                ExpressionKindDraft::Call {
                                    function: local(10),
                                    arguments: vec![result(51)],
                                },
                            ),
                        ],
                        return_value: result(52),
                    }),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::new("s55"),
                    module: local(2),
                    name: "evaluate_override".into(),
                    parameters: vec![FunctionParameterDraft {
                        symbol: DraftSymbol::new("s56"),
                        name: "value".into(),
                        ty: TypeDraft::I64,
                    }],
                    result: TypeDraft::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            expression(
                                57,
                                ExpressionKindDraft::ConstructVariant {
                                    variant: local(9),
                                    payload: Some(parameter(56)),
                                },
                            ),
                            expression(
                                58,
                                ExpressionKindDraft::Call {
                                    function: local(10),
                                    arguments: vec![result(57)],
                                },
                            ),
                        ],
                        return_value: result(58),
                    }),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::new("s60"),
                    module: local(2),
                    name: "make_reading".into(),
                    parameters: vec![
                        FunctionParameterDraft {
                            symbol: DraftSymbol::new("s61"),
                            name: "value".into(),
                            ty: TypeDraft::I64,
                        },
                        FunctionParameterDraft {
                            symbol: DraftSymbol::new("s62"),
                            name: "valid".into(),
                            ty: TypeDraft::Bool,
                        },
                    ],
                    result: TypeDraft::Nominal(local(3)),
                    body: Some(FunctionBodyDraft {
                        operations: vec![expression(
                            63,
                            ExpressionKindDraft::ConstructProduct {
                                product: local(3),
                                fields: vec![field(5, parameter(62)), field(4, parameter(61))],
                            },
                        )],
                        return_value: result(63),
                    }),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::new("s70"),
                    module: local(2),
                    name: "lazy_match_probe".into(),
                    parameters: vec![FunctionParameterDraft {
                        symbol: DraftSymbol::new("s71"),
                        name: "input".into(),
                        ty: TypeDraft::Nominal(local(6)),
                    }],
                    result: TypeDraft::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![expression(
                            72,
                            ExpressionKindDraft::MatchSum {
                                scrutinee: parameter(71),
                                result: TypeDraft::I64,
                                arms: vec![
                                    MatchArmDraft {
                                        variant: local(7),
                                        payload_symbol: Some(DraftSymbol::new("s73")),
                                        body: YieldingBodyDraft {
                                            operations: vec![expression(
                                                74,
                                                ExpressionKindDraft::ConstI64(0),
                                            )],
                                            yield_value: result(74),
                                        },
                                    },
                                    MatchArmDraft {
                                        variant: local(8),
                                        payload_symbol: None,
                                        body: YieldingBodyDraft {
                                            operations: vec![expression(
                                                75,
                                                ExpressionKindDraft::ConstI64(0),
                                            )],
                                            yield_value: result(75),
                                        },
                                    },
                                    MatchArmDraft {
                                        variant: local(9),
                                        payload_symbol: Some(DraftSymbol::new("s76")),
                                        body: YieldingBodyDraft {
                                            operations: vec![
                                                expression(
                                                    77,
                                                    ExpressionKindDraft::ConstI64(i64::MAX),
                                                ),
                                                expression(78, ExpressionKindDraft::ConstI64(1)),
                                                expression(
                                                    79,
                                                    ExpressionKindDraft::AddI64 {
                                                        lhs: result(77),
                                                        rhs: result(78),
                                                    },
                                                ),
                                            ],
                                            yield_value: result(79),
                                        },
                                    },
                                ],
                            },
                        )],
                        return_value: result(72),
                    }),
                },
                TransactionOp::CreateProductType {
                    symbol: DraftSymbol::new("s3"),
                    module: local(2),
                    name: "Reading".into(),
                    fields: vec![
                        ProductFieldDraft {
                            symbol: DraftSymbol::new("s4"),
                            name: "value".into(),
                            ty: TypeDraft::I64,
                        },
                        ProductFieldDraft {
                            symbol: DraftSymbol::new("s5"),
                            name: "valid".into(),
                            ty: TypeDraft::Bool,
                        },
                    ],
                },
                TransactionOp::CreateSumType {
                    symbol: DraftSymbol::new("s6"),
                    module: local(2),
                    name: "Input".into(),
                    variants: vec![
                        SumVariantDraft {
                            symbol: DraftSymbol::new("s7"),
                            name: "sample".into(),
                            payload: Some(TypeDraft::Nominal(local(3))),
                        },
                        SumVariantDraft {
                            symbol: DraftSymbol::new("s8"),
                            name: "missing".into(),
                            payload: None,
                        },
                        SumVariantDraft {
                            symbol: DraftSymbol::new("s9"),
                            name: "override".into(),
                            payload: Some(TypeDraft::I64),
                        },
                    ],
                },
                TransactionOp::SetEntryFunction {
                    package: local(1),
                    function: local(30),
                },
            ],
        },
        response: TransactionResponseSpec {
            return_symbols: [
                3, 4, 5, 6, 7, 8, 9, 10, 30, 31, 32, 33, 40, 50, 55, 60, 70, 79,
            ]
            .into_iter()
            .map(|value| DraftSymbol::new(&format!("s{value}")))
            .collect(),
        },
    }
}

fn reading_value(
    reading: NodeId,
    value_field: NodeId,
    valid_field: NodeId,
    value: i64,
    valid: bool,
) -> RuntimeValue {
    RuntimeValue::Product {
        ty: reading,
        // Deliberately reversed to prove identity-keyed input normalization.
        fields: vec![
            RuntimeFieldValue {
                field: valid_field,
                value: RuntimeValue::Bool(valid),
            },
            RuntimeFieldValue {
                field: value_field,
                value: RuntimeValue::I64(value),
            },
        ],
    }
}

fn input_value(input: NodeId, variant: NodeId, payload: Option<RuntimeValue>) -> RuntimeValue {
    RuntimeValue::Sum {
        ty: input,
        variant,
        payload: payload.map(Box::new),
    }
}

fn run_value(
    state: &Path,
    request_id: u64,
    workspace: WorkspaceId,
    revision: Revision,
    entry: NodeId,
    arguments: Vec<RuntimeValue>,
) -> ResponseEnvelope {
    run_value_observed(state, request_id, workspace, revision, entry, arguments).0
}

fn run_value_observed(
    state: &Path,
    request_id: u64,
    workspace: WorkspaceId,
    revision: Revision,
    entry: NodeId,
    arguments: Vec<RuntimeValue>,
) -> (ResponseEnvelope, usize) {
    rpc_observed(
        state,
        request_id,
        Request::Run {
            workspace,
            revision,
            entry,
            arguments,
            policy: lkjscript::RunPolicy {
                fuel: 1_000_000,
                maximum_frames: 1_000,
            },
        },
    )
}

#[test]
fn real_json_cli_nominal_reading_repair_application_vertical() {
    let temporary = tempfile::tempdir().expect("state directory");
    let state = temporary.path();

    let Response::DescribeSchema(manifest) = rpc(
        state,
        900,
        Request::DescribeSchema(DescribeSchemaRequest::manifest()),
    )
    .response
    else {
        panic!("schema manifest")
    };
    let DescribeSchemaResult::Manifest(manifest) = *manifest else {
        panic!("default schema projection must be manifest")
    };
    let digest = manifest.digest;
    let task_roots = vec![
        SchemaRoot::CreateWorkspace,
        SchemaRoot::ApplyTransaction,
        SchemaRoot::QueryWorkspaceSummary,
        SchemaRoot::QueryNode,
        SchemaRoot::QueryBlockers,
        SchemaRoot::QueryBody,
        SchemaRoot::QueryIncomingUses,
        SchemaRoot::QueryRepairContext,
        SchemaRoot::QuerySemanticDiff,
        SchemaRoot::QueryNominalType,
        SchemaRoot::Run,
        SchemaRoot::Shutdown,
    ];
    let Response::DescribeSchema(roots) = rpc(
        state,
        901,
        Request::DescribeSchema(DescribeSchemaRequest {
            projection: SchemaProjection::Roots {
                roots: task_roots.clone(),
            },
            known_digest: None,
        }),
    )
    .response
    else {
        panic!("schema roots")
    };
    let DescribeSchemaResult::Roots(roots) = *roots else {
        panic!("root projection")
    };
    assert_eq!(roots.digest, digest);
    assert_eq!(roots.roots.len(), task_roots.len());
    assert!(roots.definitions.len() > roots.roots.len());
    let Response::DescribeSchema(unchanged) = rpc(
        state,
        902,
        Request::DescribeSchema(DescribeSchemaRequest {
            projection: SchemaProjection::Roots { roots: task_roots },
            known_digest: Some(digest),
        }),
    )
    .response
    else {
        panic!("known digest response")
    };
    assert_eq!(*unchanged, DescribeSchemaResult::Unchanged { digest });

    let Response::WorkspaceCreated(created) = rpc(state, 903, Request::CreateWorkspace).response
    else {
        panic!("workspace")
    };
    let workspace = created.workspace;
    let creation = nominal_reading_application(workspace);
    assert_eq!(creation.transaction.operations.len(), 12);
    let created = receipt(rpc(state, 904, Request::ApplyTransaction(creation)));
    assert!(created.published);
    assert!(!created.complete_after);
    assert_eq!(created.returned_bindings.len(), 18);
    let reading = binding(&created, 3);
    let value_field = binding(&created, 4);
    let valid_field = binding(&created, 5);
    let input = binding(&created, 6);
    let sample = binding(&created, 7);
    let missing = binding(&created, 8);
    let override_variant = binding(&created, 9);
    let evaluate = binding(&created, 10);
    let main = binding(&created, 30);
    let hole = binding(&created, 33);
    let disabled = binding(&created, 40);
    let evaluate_missing = binding(&created, 50);
    let evaluate_override = binding(&created, 55);
    let make_reading = binding(&created, 60);
    let lazy_probe = binding(&created, 70);
    let overflow = binding(&created, 79);
    println!(
        "NOMINAL_READING_IDS {}",
        serde_json::json!({
            "reading": reading.serial(),
            "value_field": value_field.serial(),
            "valid_field": valid_field.serial(),
            "input": input.serial(),
            "sample": sample.serial(),
            "missing": missing.serial(),
            "override": override_variant.serial(),
            "evaluate": evaluate.serial(),
            "main": main.serial(),
            "hole": hole.serial(),
            "make_reading": make_reading.serial(),
            "lazy_probe": lazy_probe.serial(),
            "overflow_origin": overflow.serial(),
        })
    );

    let QueryResult::RepairContext(context) = query(
        state,
        905,
        workspace,
        Revision::new(1),
        Query::RepairContext {
            target: RepairTarget::Hole(hole),
            budget: ContextBudget {
                body_before: 8,
                body_after: 8,
                visible_values: 16,
                incoming_uses: 8,
                include_incompatible: true,
            },
        },
    ) else {
        panic!("Reading repair context")
    };
    assert_eq!(context.expected_type, SemanticType::Nominal(reading));
    assert_eq!(context.operation, hole);
    assert_eq!(context.owner_function, main);
    let nominal = context.nominal_type.as_ref().expect("nominal context");
    assert_eq!(nominal.declaration, reading);
    assert_eq!(nominal.name, "Reading");
    assert_eq!(nominal.kind, NodeKind::ProductType);
    assert_eq!(nominal.members.items.len(), 2);
    assert!(matches!(
        nominal.members.items[0],
        lkjscript::query::NominalMemberFact::ProductField {
            field,
            ordinal: 0,
            ref name,
            ty: SemanticType::I64,
            ..
        } if field == value_field && name == "value"
    ));
    assert!(matches!(
        nominal.members.items[1],
        lkjscript::query::NominalMemberFact::ProductField {
            field,
            ordinal: 1,
            ref name,
            ty: SemanticType::Bool,
            ..
        } if field == valid_field && name == "valid"
    ));
    assert!(context.visible_values.items.iter().any(|visible| {
        visible.ty == SemanticType::I64
            && visible.producer_code == Some(OperationCode::ConstI64)
            && visible.producer == binding(&created, 31)
    }));
    assert!(context.visible_values.items.iter().any(|visible| {
        visible.ty == SemanticType::Bool
            && visible.producer_code == Some(OperationCode::ConstBool)
            && visible.producer == binding(&created, 32)
    }));
    assert!(context.body_window.iter().any(|item| {
        item.operation == binding(&created, 31)
            && item.literal == Some(lkjscript::query::LiteralValue::I64(42))
    }));
    assert!(context.body_window.iter().any(|item| {
        item.operation == binding(&created, 32)
            && item.literal == Some(lkjscript::query::LiteralValue::Bool(true))
    }));
    let product_constructor = context
        .legal_constructors
        .iter()
        .find(|constructor| constructor.code == OperationCode::ConstructProduct)
        .expect("construct_product repair contract");
    assert_eq!(product_constructor.declaration, Some(reading));
    assert_eq!(
        product_constructor.result_type,
        SemanticType::Nominal(reading)
    );
    assert_eq!(product_constructor.operand_count, 2);
    assert_eq!(
        product_constructor.operand_types,
        vec![SemanticType::I64, SemanticType::Bool]
    );
    assert_eq!(product_constructor.members, vec![value_field, valid_field]);
    assert!(product_constructor.direct_refinement);
    let sample_use = context
        .incoming_uses
        .items
        .iter()
        .find(|site| site.expected_type == SemanticType::Nominal(reading))
        .expect("existing sample use");
    let sample_operation = sample_use.source;
    assert!(context.body_window.iter().any(|item| {
        item.operation == sample_operation
            && item.code == OperationCode::ConstructVariant
            && item
                .definitions
                .iter()
                .any(|definition| definition.target == sample)
    }));
    assert_eq!(
        context.blocker.as_ref().and_then(|item| item.target),
        Some(hole)
    );
    let owner_block = context.owner_block;
    let body_ordinal = context.ordinal;

    let workspace_dir = workspace_path(state, workspace);
    let head_path = workspace_dir.join("HEAD");
    let artifact_path = workspace_dir.join("revisions/00000000000000000001.lkjscript");
    let head_before_invalid = fs::read(&head_path).expect("HEAD before invalid repair");
    let artifact_before_invalid = fs::read(&artifact_path).expect("artifact before invalid repair");
    let files_before_invalid = revision_files(state, workspace);
    let allocation_probe = ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::ValidateOnly,
            operations: vec![TransactionOp::CreatePackage {
                symbol: DraftSymbol::new("s1"),
                name: "allocator-frontier-probe".into(),
            }],
        },
        response: TransactionResponseSpec {
            return_symbols: vec![DraftSymbol::new("s1")],
        },
    };
    let probe_before = receipt(rpc(
        state,
        895,
        Request::ApplyTransaction(allocation_probe.clone()),
    ));
    assert!(!probe_before.published);
    assert_eq!(probe_before.base_revision, Revision::new(1));
    assert_eq!(probe_before.revision, Revision::new(2));
    assert_eq!(probe_before.created_count, 1);
    assert_eq!(probe_before.returned_bindings.len(), 1);
    assert_eq!(probe_before.returned_bindings[0].0, DraftSymbol::new("s1"));
    assert_eq!(
        fs::read(&head_path).expect("HEAD after first allocation probe"),
        head_before_invalid
    );
    assert_eq!(
        fs::read(&artifact_path).expect("artifact after first allocation probe"),
        artifact_before_invalid
    );
    assert_eq!(revision_files(state, workspace), files_before_invalid);
    let invalid = ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::ConstructProduct {
                    product: NodeTarget::Existing(reading),
                    fields: vec![
                        ProductFieldValueDraft {
                            field: NodeTarget::Existing(valid_field),
                            value: existing_result(binding(&created, 31)),
                        },
                        ProductFieldValueDraft {
                            field: NodeTarget::Existing(value_field),
                            value: existing_result(binding(&created, 32)),
                        },
                    ],
                },
            }],
        },
        response: TransactionResponseSpec::default(),
    };
    let Response::Error(invalid) = rpc(state, 906, Request::ApplyTransaction(invalid)).response
    else {
        panic!("wrong field values must reject")
    };
    assert_eq!(invalid.code, ErrorCode::TypeMismatch);
    assert_eq!(invalid.expected_type, Some(SemanticType::I64));
    assert_eq!(invalid.actual_type, Some(SemanticType::Bool));
    assert_eq!(
        fs::read(&head_path).expect("HEAD after rejection"),
        head_before_invalid
    );
    assert_eq!(
        fs::read(&artifact_path).expect("artifact after rejection"),
        artifact_before_invalid
    );
    assert_eq!(revision_files(state, workspace), files_before_invalid);
    let probe_after = receipt(rpc(state, 896, Request::ApplyTransaction(allocation_probe)));
    assert!(!probe_after.published);
    assert_eq!(
        probe_after.returned_bindings,
        probe_before.returned_bindings
    );
    assert_eq!(probe_after.created_count, probe_before.created_count);
    assert_eq!(probe_after.hash, probe_before.hash);
    assert_eq!(probe_after.change_count, probe_before.change_count);
    assert_eq!(probe_after.change_digest, probe_before.change_digest);
    assert_eq!(probe_after, probe_before);
    assert_eq!(
        fs::read(&head_path).expect("HEAD after second allocation probe"),
        head_before_invalid
    );
    assert_eq!(
        fs::read(&artifact_path).expect("artifact after second allocation probe"),
        artifact_before_invalid
    );
    assert_eq!(revision_files(state, workspace), files_before_invalid);
    let QueryResult::Node(still_hole) = query(
        state,
        907,
        workspace,
        Revision::new(1),
        Query::Node {
            node: hole,
            expand: true,
        },
    ) else {
        panic!("hole after rejected repair")
    };
    assert_eq!(still_hole.summary.kind, NodeKind::Operation);
    let Response::Run(still_usable) = run_value(
        state,
        908,
        workspace,
        Revision::new(1),
        evaluate_missing,
        vec![],
    )
    .response
    else {
        panic!("engine usability after rejection")
    };
    assert_eq!(still_usable.value, RuntimeValue::I64(0));

    let valid = ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::new(1),
            idempotency_key: Some(IdempotencyKey::from_bytes([0x82; 16])),
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::ConstructProduct {
                    product: NodeTarget::Existing(reading),
                    fields: vec![
                        ProductFieldValueDraft {
                            field: NodeTarget::Existing(valid_field),
                            value: existing_result(binding(&created, 32)),
                        },
                        ProductFieldValueDraft {
                            field: NodeTarget::Existing(value_field),
                            value: existing_result(binding(&created, 31)),
                        },
                    ],
                },
            }],
        },
        response: TransactionResponseSpec::default(),
    };
    let refined = receipt(rpc(state, 909, Request::ApplyTransaction(valid)));
    assert_eq!(refined.revision, Revision::new(2));
    assert_eq!(refined.created_count, 0);
    assert!(refined.complete_after);
    let QueryResult::Body(body) = query(
        state,
        910,
        workspace,
        Revision::new(2),
        Query::Body {
            block: owner_block,
            page: PageRequest {
                after: None,
                limit: 16,
            },
        },
    ) else {
        panic!("repaired body")
    };
    let repaired_hole = body
        .items
        .iter()
        .find(|item| item.operation == hole)
        .expect("preserved hole identity");
    assert_eq!(repaired_hole.ordinal, body_ordinal);
    assert_eq!(repaired_hole.code, OperationCode::ConstructProduct);
    let QueryResult::IncomingUses(repaired_uses) = query(
        state,
        911,
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
        panic!("repaired uses")
    };
    assert!(
        repaired_uses
            .items
            .iter()
            .any(|site| site.source == sample_operation)
    );

    let diff = collect_diff(state, workspace, Revision::new(1), Revision::new(2));
    assert_eq!(diff.0, refined.change_count);
    assert_eq!(diff.1, refined.change_digest);
    assert_eq!(diff.2.len(), 4);
    assert!(diff.2.iter().any(|change| {
        change.node == hole
            && matches!(
                change.kind,
                ChangeKind::OperationRefined {
                    before: OperationCode::Hole,
                    after: OperationCode::ConstructProduct,
                    result_type: SemanticType::Nominal(declaration),
                    ..
                } if declaration == reading
            )
    }));
    assert!(diff.2.iter().all(|change| !matches!(
        change.kind,
        ChangeKind::Created { .. } | ChangeKind::Deleted { .. }
    )));

    let repaired_head_path = workspace_dir.join("HEAD");
    let repaired_artifact_path = workspace_dir.join("revisions/00000000000000000002.lkjscript");
    let repaired_head = fs::read(&repaired_head_path).expect("repaired HEAD");
    let repaired_artifact = fs::read(&repaired_artifact_path).expect("repaired artifact");
    let repaired_revision_files = revision_files(state, workspace);
    let foreign_workspace = WorkspaceId::from_bytes([0xa5; 16]);
    let foreign_node = NodeId::new(foreign_workspace, 1).expect("foreign node identity");
    let malformed_runs = vec![
        (
            "product_missing_field",
            input_value(
                input,
                sample,
                Some(RuntimeValue::Product {
                    ty: reading,
                    fields: vec![RuntimeFieldValue {
                        field: value_field,
                        value: RuntimeValue::I64(5),
                    }],
                }),
            ),
            ErrorCode::RunArgumentMismatch,
        ),
        (
            "product_duplicate_field",
            input_value(
                input,
                sample,
                Some(RuntimeValue::Product {
                    ty: reading,
                    fields: vec![
                        RuntimeFieldValue {
                            field: value_field,
                            value: RuntimeValue::I64(5),
                        },
                        RuntimeFieldValue {
                            field: value_field,
                            value: RuntimeValue::I64(6),
                        },
                    ],
                }),
            ),
            ErrorCode::RunArgumentMismatch,
        ),
        (
            "product_foreign_field",
            input_value(
                input,
                sample,
                Some(RuntimeValue::Product {
                    ty: reading,
                    fields: vec![
                        RuntimeFieldValue {
                            field: value_field,
                            value: RuntimeValue::I64(5),
                        },
                        RuntimeFieldValue {
                            field: foreign_node,
                            value: RuntimeValue::Bool(true),
                        },
                    ],
                }),
            ),
            ErrorCode::WrongWorkspace,
        ),
        (
            "product_wrong_kind_field",
            input_value(
                input,
                sample,
                Some(RuntimeValue::Product {
                    ty: reading,
                    fields: vec![
                        RuntimeFieldValue {
                            field: value_field,
                            value: RuntimeValue::I64(5),
                        },
                        RuntimeFieldValue {
                            field: sample,
                            value: RuntimeValue::Bool(true),
                        },
                    ],
                }),
            ),
            ErrorCode::RunArgumentMismatch,
        ),
        (
            "product_wrong_nested_field_type",
            input_value(
                input,
                sample,
                Some(RuntimeValue::Product {
                    ty: reading,
                    fields: vec![
                        RuntimeFieldValue {
                            field: value_field,
                            value: RuntimeValue::Bool(true),
                        },
                        RuntimeFieldValue {
                            field: valid_field,
                            value: RuntimeValue::Bool(true),
                        },
                    ],
                }),
            ),
            ErrorCode::RunArgumentMismatch,
        ),
        (
            "nullary_variant_with_payload",
            input_value(input, missing, Some(RuntimeValue::I64(1))),
            ErrorCode::RunArgumentMismatch,
        ),
        (
            "payload_variant_omitted",
            input_value(input, override_variant, None),
            ErrorCode::RunArgumentMismatch,
        ),
        (
            "wrong_variant_payload_type",
            input_value(input, override_variant, Some(RuntimeValue::Bool(true))),
            ErrorCode::RunArgumentMismatch,
        ),
        (
            "foreign_variant",
            input_value(input, foreign_node, None),
            ErrorCode::WrongWorkspace,
        ),
        (
            "wrong_kind_variant",
            input_value(input, value_field, None),
            ErrorCode::RunArgumentMismatch,
        ),
        (
            "wrong_kind_sum_type",
            input_value(reading, sample, None),
            ErrorCode::RunArgumentMismatch,
        ),
        (
            "foreign_sum_type",
            input_value(foreign_node, sample, None),
            ErrorCode::RunArgumentMismatch,
        ),
    ];
    for (index, (name, argument, expected_code)) in malformed_runs.into_iter().enumerate() {
        let (response, stdout_bytes) = run_value_observed(
            state,
            2_000 + u64::try_from(index).expect("case index"),
            workspace,
            Revision::new(2),
            evaluate,
            vec![argument],
        );
        let Response::Error(error) = response.response else {
            panic!("malformed nominal Run {name} must return a typed semantic error")
        };
        assert_eq!(error.code, expected_code, "malformed nominal Run {name}");
        assert!(!error.retryable, "malformed nominal Run {name}");
        assert!(
            error.message.len() <= 256,
            "bounded error message for {name}"
        );
        assert!(error.related.len() <= 64, "bounded related IDs for {name}");
        assert!(stdout_bytes < 4 * 1024, "bounded error response for {name}");
        assert_eq!(
            fs::read(&repaired_head_path).expect("HEAD after malformed Run"),
            repaired_head,
            "malformed nominal Run {name} must not mutate HEAD"
        );
        assert_eq!(
            fs::read(&repaired_artifact_path).expect("artifact after malformed Run"),
            repaired_artifact,
            "malformed nominal Run {name} must not mutate the artifact"
        );
        assert_eq!(
            revision_files(state, workspace),
            repaired_revision_files,
            "malformed nominal Run {name} must not publish a revision"
        );
    }
    let valid_after_malformed = input_value(
        input,
        sample,
        Some(reading_value(reading, value_field, valid_field, 5, true)),
    );
    let Response::Run(valid_after_malformed) = run_value(
        state,
        2_100,
        workspace,
        Revision::new(2),
        evaluate,
        vec![valid_after_malformed],
    )
    .response
    else {
        panic!("valid nominal Run after malformed inputs")
    };
    assert_eq!(valid_after_malformed.value, RuntimeValue::I64(5));
    let QueryResult::Node(still_repaired) = query(
        state,
        2_101,
        workspace,
        Revision::new(2),
        Query::Node {
            node: hole,
            expand: false,
        },
    ) else {
        panic!("repaired node after malformed nominal Runs")
    };
    assert!(still_repaired.summary.complete);
    assert_eq!(still_repaired.summary.revision, Revision::new(2));

    let run_i64 = |id, entry, arguments, expected| {
        let Response::Run(result) =
            run_value(state, id, workspace, Revision::new(2), entry, arguments).response
        else {
            panic!("i64 Run oracle")
        };
        assert_eq!(result.value, RuntimeValue::I64(expected));
    };
    run_i64(920, main, vec![], 42);
    run_i64(921, disabled, vec![RuntimeValue::I64(17)], 0);
    run_i64(922, evaluate_missing, vec![], 0);
    run_i64(923, evaluate_override, vec![RuntimeValue::I64(7)], 7);
    let expected_reading = RuntimeValue::Product {
        ty: reading,
        fields: vec![
            RuntimeFieldValue {
                field: value_field,
                value: RuntimeValue::I64(9),
            },
            RuntimeFieldValue {
                field: valid_field,
                value: RuntimeValue::Bool(true),
            },
        ],
    };
    let Response::Run(made) = run_value(
        state,
        924,
        workspace,
        Revision::new(2),
        make_reading,
        vec![RuntimeValue::I64(9), RuntimeValue::Bool(true)],
    )
    .response
    else {
        panic!("nominal output")
    };
    assert_eq!(made.value, expected_reading);
    let missing_input = input_value(input, missing, None);
    let override_input = input_value(input, override_variant, Some(RuntimeValue::I64(11)));
    let sample_true = input_value(
        input,
        sample,
        Some(reading_value(reading, value_field, valid_field, 5, true)),
    );
    let sample_false = input_value(
        input,
        sample,
        Some(reading_value(reading, value_field, valid_field, 5, false)),
    );
    run_i64(925, evaluate, vec![missing_input.clone()], 0);
    run_i64(926, evaluate, vec![override_input], 11);
    run_i64(927, evaluate, vec![sample_true.clone()], 5);
    run_i64(928, evaluate, vec![sample_false], 0);
    run_i64(929, lazy_probe, vec![missing_input.clone()], 0);
    let Response::Error(trap) = run_value(
        state,
        930,
        workspace,
        Revision::new(2),
        lazy_probe,
        vec![input_value(
            input,
            override_variant,
            Some(RuntimeValue::I64(0)),
        )],
    )
    .response
    else {
        panic!("selected overflow must trap")
    };
    assert_eq!(trap.code, ErrorCode::RuntimeTrap);
    assert_eq!(trap.target, Some(overflow));
    run_i64(931, lazy_probe, vec![missing_input.clone()], 0);

    shutdown(state, 932);
    for revision in [Revision::new(1), Revision::new(2)] {
        for node in [
            reading,
            value_field,
            valid_field,
            input,
            sample,
            missing,
            override_variant,
            hole,
        ] {
            let QueryResult::Node(view) = query(
                state,
                940 + revision.get() + node.serial(),
                workspace,
                revision,
                Query::Node {
                    node,
                    expand: false,
                },
            ) else {
                panic!("retained nominal identity")
            };
            assert_eq!(view.summary.node, node);
            assert_eq!(view.summary.revision, revision);
            if node == hole {
                assert_eq!(view.summary.complete, revision == Revision::new(2));
                assert_eq!(
                    view.summary.value_type,
                    Some(SemanticType::Nominal(reading))
                );
            }
        }
        let QueryResult::NominalType(reading_context) = query(
            state,
            980 + revision.get(),
            workspace,
            revision,
            Query::NominalType {
                declaration: reading,
                page: PageRequest {
                    after: None,
                    limit: 8,
                },
            },
        ) else {
            panic!("retained Reading context")
        };
        assert_eq!(reading_context.name, "Reading");
        assert_eq!(reading_context.members.items.len(), 2);
        let QueryResult::NominalType(input_context) = query(
            state,
            990 + revision.get(),
            workspace,
            revision,
            Query::NominalType {
                declaration: input,
                page: PageRequest {
                    after: None,
                    limit: 8,
                },
            },
        ) else {
            panic!("retained Input context")
        };
        assert_eq!(input_context.name, "Input");
        assert_eq!(input_context.members.items.len(), 3);
    }
    let Response::Error(incomplete) =
        run_value(state, 1000, workspace, Revision::new(1), main, vec![]).response
    else {
        panic!("retained incomplete Run")
    };
    assert_eq!(incomplete.code, ErrorCode::CompileIncomplete);
    let Response::Run(main_after_restart) =
        run_value(state, 1001, workspace, Revision::new(2), main, vec![]).response
    else {
        panic!("main after restart")
    };
    assert_eq!(main_after_restart.value, RuntimeValue::I64(42));
    let Response::Run(sample_after_restart) = run_value(
        state,
        1002,
        workspace,
        Revision::new(2),
        evaluate,
        vec![sample_true],
    )
    .response
    else {
        panic!("nominal input after restart")
    };
    assert_eq!(sample_after_restart.value, RuntimeValue::I64(5));
    let Response::Run(output_after_restart) = run_value(
        state,
        1003,
        workspace,
        Revision::new(2),
        make_reading,
        vec![RuntimeValue::I64(9), RuntimeValue::Bool(true)],
    )
    .response
    else {
        panic!("nominal output after restart")
    };
    assert_eq!(output_after_restart.value, expected_reading);
    shutdown(state, 1004);
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

fn local_schema() -> DescribeSchemaResult {
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
