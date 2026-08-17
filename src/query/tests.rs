use super::*;
use crate::graph::Workspace;
use crate::ids::{DraftSymbol, WorkspaceId};
use crate::schema::{OperationDraft, TypeDraft, ValueDraft};
use crate::transaction::{
    ApplyTransactionRequest, ExpressionDraft, ExpressionKindDraft, FunctionBodyDraft,
    FunctionParameterDraft, NodeTarget, ProductFieldDraft, Transaction, TransactionMode,
    TransactionOp, TransactionResponseSpec, YieldingBodyDraft,
};

#[test]
fn byte_hole_repair_context_exposes_exact_type_and_direct_constructors() {
    let id = WorkspaceId::from_bytes([0xb6; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let local = |value| NodeTarget::Draft(DraftSymbol::generated(value));
    let request = ApplyTransactionRequest {
        transaction: Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: local(2),
                    name: "repair_bytes".into(),
                    parameters: vec![FunctionParameterDraft {
                        symbol: DraftSymbol::generated(4),
                        name: "input".into(),
                        ty: TypeDraft::Bytes,
                    }],
                    result: TypeDraft::Bytes,
                    body: Some(FunctionBodyDraft {
                        operations: vec![ExpressionDraft {
                            symbol: Some(DraftSymbol::generated(5)),
                            operation: ExpressionKindDraft::Hole {
                                expected: TypeDraft::Bytes,
                            },
                        }],
                        return_value: ValueDraft::OperationResult {
                            operation: local(5),
                            output: 0,
                        },
                    }),
                },
            ],
        },
        response: TransactionResponseSpec {
            return_symbols: vec![DraftSymbol::generated(5)],
        },
    };
    let prepared = workspace
        .prepare_transaction(&request)
        .expect("byte hole proposal");
    let hole = prepared.receipt.returned_bindings[0].1;
    let context = repair_context(
        &prepared.snapshot,
        RepairTarget::Hole(hole),
        ContextBudget {
            body_before: 4,
            body_after: 4,
            visible_values: 8,
            incoming_uses: 8,
            include_incompatible: true,
        },
    )
    .expect("byte repair context");
    assert_eq!(context.expected_type, SemanticType::Bytes);
    for code in [
        OperationCode::ConstBytes,
        OperationCode::BytesSlice,
        OperationCode::BytesConcat,
    ] {
        assert!(
            context
                .legal_constructors
                .iter()
                .any(|constructor| constructor.code == code && constructor.direct_refinement),
            "missing direct byte constructor {code:?}"
        );
    }
    assert!(
        context
            .visible_values
            .items
            .iter()
            .any(|candidate| { candidate.ty == SemanticType::Bytes && candidate.compatible })
    );
}

fn fixture() -> (Workspace, Vec<NodeId>) {
    let id = WorkspaceId::from_bytes([0x66; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let local = |v| NodeTarget::Draft(DraftSymbol::generated(v));
    let value = |v| ValueDraft::OperationResult {
        operation: local(v),
        output: 0,
    };
    let tx = Transaction {
        workspace: id,
        base_revision: Revision::INITIAL,
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![
            TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(1),
                name: "app".into(),
            },
            TransactionOp::CreateModule {
                symbol: DraftSymbol::generated(2),
                package: local(1),
                name: "root".into(),
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: local(2),
                name: "main".into(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: vec![
                        ExpressionDraft {
                            symbol: Some(DraftSymbol::generated(6)),
                            operation: ExpressionKindDraft::ConstI64(40),
                        },
                        ExpressionDraft {
                            symbol: Some(DraftSymbol::generated(7)),
                            operation: ExpressionKindDraft::ConstI64(2),
                        },
                        ExpressionDraft {
                            symbol: Some(DraftSymbol::generated(8)),
                            operation: ExpressionKindDraft::ConstBool(true),
                        },
                        ExpressionDraft {
                            symbol: Some(DraftSymbol::generated(9)),
                            operation: ExpressionKindDraft::Hole {
                                expected: SemanticType::I64.into(),
                            },
                        },
                    ],
                    return_value: value(9),
                }),
            },
            TransactionOp::SetEntryFunction {
                package: local(1),
                function: local(3),
            },
        ],
    };
    let request = ApplyTransactionRequest {
        transaction: tx,
        response: TransactionResponseSpec {
            return_symbols: [1, 2, 3, 6, 7, 8, 9]
                .into_iter()
                .map(DraftSymbol::generated)
                .collect(),
        },
    };
    let prepared = workspace.prepare_transaction(&request).expect("prepare");
    let binding = |symbol| {
        prepared
            .receipt
            .returned_bindings
            .iter()
            .find_map(|(candidate, id)| {
                (*candidate == DraftSymbol::generated(symbol)).then_some(*id)
            })
            .expect("binding")
    };
    let function = binding(3);
    let Node::Function {
        body: Some(region), ..
    } = prepared.snapshot.node(function).expect("function")
    else {
        panic!("function body")
    };
    let Node::Region { blocks, .. } = prepared.snapshot.node(*region).expect("region") else {
        panic!("region")
    };
    let block = blocks[0];
    let Node::Block {
        terminator: Some(terminator),
        ..
    } = prepared.snapshot.node(block).expect("block")
    else {
        panic!("terminator")
    };
    let ids = vec![
        binding(1),
        binding(2),
        function,
        *region,
        block,
        binding(6),
        binding(7),
        binding(8),
        binding(9),
        *terminator,
    ];
    workspace.publish(prepared.snapshot).expect("publish");
    (workspace, ids)
}

#[test]
fn pages_uses_visibility_constructors_and_context_are_exact() {
    let (workspace, ids) = fixture();
    let snapshot = workspace.head().expect("head");
    let block = ids[4];
    let forty = ids[5];
    let two = ids[6];
    let boolean = ids[7];
    let hole = ids[8];
    let ret = ids[9];

    let owner_first = owner_chain_page(
        snapshot,
        hole,
        PageRequest {
            after: None,
            limit: 2,
        },
    )
    .expect("owner first");
    assert_eq!(owner_first.items.len(), 2);
    assert_eq!(owner_first.items[0].node, hole);
    assert!(owner_first.total.expect("owner total") > 2);
    let owner_second = owner_chain_page(
        snapshot,
        hole,
        PageRequest {
            after: owner_first.next,
            limit: 2,
        },
    )
    .expect("owner second");
    assert_eq!(owner_second.items.len(), 2);
    let mut wrong_owner_cursor = owner_first.next.expect("owner cursor");
    if let PageCursor::OwnerChain { node, .. } = &mut wrong_owner_cursor {
        *node = boolean;
    }
    assert_eq!(
        owner_chain_page(
            snapshot,
            hole,
            PageRequest {
                after: Some(wrong_owner_cursor),
                limit: 2,
            },
        )
        .expect_err("bound owner cursor")
        .code,
        ErrorCode::InvalidCursor
    );

    let first = body_page(
        snapshot,
        block,
        PageRequest {
            after: None,
            limit: 2,
        },
    )
    .expect("body page");
    assert_eq!(first.items.len(), 2);
    let next = first.next.expect("next");
    let rest = body_page(
        snapshot,
        block,
        PageRequest {
            after: Some(next),
            limit: MAX_PAGE_ITEMS,
        },
    )
    .expect("body rest");
    assert_eq!(rest.items.last().map(|x| x.operation), Some(ret));
    assert!(rest.items.last().expect("return").terminator);
    assert!(rest.next.is_none());
    let terminal = body_page(
        snapshot,
        block,
        PageRequest {
            after: Some(PageCursor::Body {
                workspace: snapshot.workspace(),
                revision: snapshot.revision(),
                block,
                next: 5,
            }),
            limit: 1,
        },
    )
    .expect("terminal cursor");
    assert!(terminal.items.is_empty());
    assert!(terminal.next.is_none());
    assert_eq!(
        body_page(
            snapshot,
            block,
            PageRequest {
                after: Some(PageCursor::Body {
                    workspace: snapshot.workspace(),
                    revision: snapshot.revision(),
                    block,
                    next: 99
                }),
                limit: 1
            }
        )
        .expect_err("beyond")
        .code,
        ErrorCode::InvalidCursor
    );
    assert_eq!(
        body_page(
            snapshot,
            block,
            PageRequest {
                after: None,
                limit: 0
            }
        )
        .expect_err("zero")
        .code,
        ErrorCode::InvalidQuery
    );
    let uses = uses_page(
        snapshot,
        ValueRef::OperationResult {
            operation: hole,
            output: 0,
        },
        PageRequest {
            after: None,
            limit: 8,
        },
    )
    .expect("uses");
    assert_eq!(uses.items.len(), 1);
    assert_eq!(uses.items[0].source, ret);
    assert_eq!(uses.items[0].operand_index, 0);
    let crossed_parameter = value_type(snapshot, ValueRef::FunctionParameter(hole))
        .expect_err("operation as parameter");
    assert_eq!(
        (
            crossed_parameter.code,
            crossed_parameter.expected_kind,
            crossed_parameter.actual_kind
        ),
        (
            ErrorCode::WrongKind,
            Some(NodeKind::Parameter),
            Some(NodeKind::Operation)
        )
    );
    let crossed_operation = value_type(
        snapshot,
        ValueRef::OperationResult {
            operation: ids[2],
            output: 0,
        },
    )
    .expect_err("function as operation");
    assert_eq!(
        (
            crossed_operation.code,
            crossed_operation.expected_kind,
            crossed_operation.actual_kind
        ),
        (
            ErrorCode::WrongKind,
            Some(NodeKind::Operation),
            Some(NodeKind::Function)
        )
    );
    assert_eq!(
        value_type(
            snapshot,
            ValueRef::OperationResult {
                operation: hole,
                output: 1
            }
        )
        .expect_err("invalid output")
        .code,
        ErrorCode::InvalidOperand
    );
    let (expected, loc) = target_contract(snapshot, RepairTarget::Hole(hole)).expect("contract");
    let visible = visible_page(
        snapshot,
        VisibleCursorPurpose::VisibleValues,
        RepairTarget::Hole(hole),
        expected,
        loc,
        true,
        PageRequest {
            after: None,
            limit: 8,
        },
    )
    .expect("visible");
    assert_eq!(
        visible.items.iter().map(|v| v.producer).collect::<Vec<_>>(),
        vec![forty, two, boolean]
    );
    assert!(!visible.items[2].compatible);
    assert_eq!(
        legal_constructor_slice(snapshot, SemanticType::I64, 0, MAX_CONTEXT_ITEMS as usize)
            .0
            .iter()
            .map(|c| c.code)
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
    assert_eq!(
        legal_constructor_slice(snapshot, SemanticType::Bool, 0, MAX_CONTEXT_ITEMS as usize)
            .0
            .iter()
            .map(|c| c.code)
            .collect::<Vec<_>>(),
        vec![
            OperationCode::ConstBool,
            OperationCode::LtI64,
            OperationCode::If,
            OperationCode::ForI64,
            OperationCode::BytesEqual,
        ]
    );
    assert_eq!(
        legal_constructor_slice(snapshot, SemanticType::Unit, 0, MAX_CONTEXT_ITEMS as usize)
            .0
            .iter()
            .map(|c| c.code)
            .collect::<Vec<_>>(),
        vec![
            OperationCode::ConstUnit,
            OperationCode::If,
            OperationCode::ForI64,
        ]
    );
    let context = repair_context(
        snapshot,
        RepairTarget::Hole(hole),
        ContextBudget {
            body_before: 2,
            body_after: 1,
            visible_values: 8,
            incoming_uses: 8,
            include_incompatible: true,
        },
    )
    .expect("context");
    assert_eq!(context.expected_type, SemanticType::I64);
    assert_eq!(context.incoming_uses.items[0].source, ret);
    assert_eq!(
        context.refinement_operation,
        Some(TransactionOpCode::RefineHole)
    );
    assert!(
        context
            .visible_values
            .items
            .iter()
            .any(|v| v.producer == boolean && !v.compatible)
    );
    let zero_context = repair_context(
        snapshot,
        RepairTarget::Hole(hole),
        ContextBudget {
            body_before: 0,
            body_after: 0,
            visible_values: 0,
            incoming_uses: 0,
            include_incompatible: true,
        },
    )
    .expect("zero context");
    assert!(zero_context.visible_values.items.is_empty());
    let visible_cursor = zero_context
        .visible_values
        .next
        .expect("zero visible continuation");
    assert!(matches!(
        visible_cursor,
        PageCursor::VisibleValues {
            purpose: VisibleCursorPurpose::RepairContext,
            next: 0,
            ..
        }
    ));
    let continued_visible = visible_page(
        snapshot,
        VisibleCursorPurpose::RepairContext,
        RepairTarget::Hole(hole),
        expected,
        loc,
        true,
        PageRequest {
            after: Some(visible_cursor),
            limit: 1,
        },
    )
    .expect("context visible continuation");
    assert_eq!(continued_visible.items[0].producer, forty);
    for purpose in [
        VisibleCursorPurpose::VisibleValues,
        VisibleCursorPurpose::LegalConstructors,
    ] {
        assert_eq!(
            visible_page(
                snapshot,
                purpose,
                RepairTarget::Hole(hole),
                expected,
                loc,
                true,
                PageRequest {
                    after: Some(visible_cursor),
                    limit: 1
                }
            )
            .expect_err("cross-purpose visible cursor")
            .code,
            ErrorCode::InvalidCursor
        );
    }
    assert_eq!(
        visible_page(
            snapshot,
            VisibleCursorPurpose::RepairContext,
            RepairTarget::Hole(hole),
            expected,
            loc,
            false,
            PageRequest {
                after: Some(visible_cursor),
                limit: 1
            }
        )
        .expect_err("cross-option visible cursor")
        .code,
        ErrorCode::InvalidCursor
    );
    assert_eq!(
        visible_page(
            snapshot,
            VisibleCursorPurpose::RepairContext,
            RepairTarget::Operand {
                operation: ret,
                index: 0
            },
            expected,
            operation_location(snapshot, ret).expect("return location"),
            true,
            PageRequest {
                after: Some(visible_cursor),
                limit: 1
            }
        )
        .expect_err("cross-target visible cursor")
        .code,
        ErrorCode::InvalidCursor
    );
    let wrong_revision = match visible_cursor {
        PageCursor::VisibleValues {
            workspace,
            purpose,
            target,
            expected,
            include_incompatible,
            next,
            ..
        } => PageCursor::VisibleValues {
            workspace,
            revision: Revision::new(2),
            purpose,
            target,
            expected,
            include_incompatible,
            next,
        },
        _ => unreachable!(),
    };
    assert_eq!(
        visible_page(
            snapshot,
            VisibleCursorPurpose::RepairContext,
            RepairTarget::Hole(hole),
            expected,
            loc,
            true,
            PageRequest {
                after: Some(wrong_revision),
                limit: 1
            }
        )
        .expect_err("cross-revision visible cursor")
        .code,
        ErrorCode::InvalidCursor
    );
    assert_eq!(
        uses_page(
            snapshot,
            ValueRef::OperationResult {
                operation: hole,
                output: 0
            },
            PageRequest {
                after: Some(visible_cursor),
                limit: 1
            }
        )
        .expect_err("cross-family cursor")
        .code,
        ErrorCode::InvalidCursor
    );
    assert!(zero_context.incoming_uses.items.is_empty());
    let incoming_cursor = zero_context
        .incoming_uses
        .next
        .expect("zero incoming continuation");
    assert!(matches!(
        incoming_cursor,
        PageCursor::IncomingUses { next: 0, .. }
    ));
    // Context incoming-use cursors intentionally continue through the exact public IncomingUses query.
    assert_eq!(
        uses_page(
            snapshot,
            ValueRef::OperationResult {
                operation: hole,
                output: 0
            },
            PageRequest {
                after: Some(incoming_cursor),
                limit: 1
            }
        )
        .expect("context incoming continuation")
        .items[0]
            .source,
        ret
    );
    assert_eq!(
        uses_page(
            snapshot,
            ValueRef::OperationResult {
                operation: forty,
                output: 0
            },
            PageRequest {
                after: Some(incoming_cursor),
                limit: 1
            }
        )
        .expect_err("cross-target incoming cursor")
        .code,
        ErrorCode::InvalidCursor
    );
    let definitions = definition_page(
        snapshot,
        ids[2],
        PageRequest {
            after: None,
            limit: 8,
        },
    )
    .expect("definition refs");
    assert_eq!(definitions.items.len(), 1);
    assert_eq!(definitions.items[0].source, ids[0]);
    assert_eq!(
        dependencies(snapshot, ret).expect("return deps"),
        vec![DependencyFact::ValueOperand {
            index: 0,
            value: ValueRef::OperationResult {
                operation: hole,
                output: 0
            }
        }]
    );
    let operand = repair_context(
        snapshot,
        RepairTarget::Operand {
            operation: ret,
            index: 0,
        },
        ContextBudget {
            body_before: 2,
            body_after: 0,
            visible_values: 8,
            incoming_uses: 8,
            include_incompatible: true,
        },
    )
    .expect("operand context");
    assert_eq!(
        operand.current_value,
        Some(ValueRef::OperationResult {
            operation: hole,
            output: 0
        })
    );
    assert!(
        operand
            .visible_values
            .items
            .iter()
            .any(|v| v.producer == boolean && !v.compatible)
    );
    let cross = PageCursor::Body {
        workspace: WorkspaceId::from_bytes([1; 16]),
        revision: snapshot.revision(),
        block,
        next: 0,
    };
    assert_eq!(
        body_page(
            snapshot,
            block,
            PageRequest {
                after: Some(cross),
                limit: 1
            }
        )
        .expect_err("cross workspace")
        .code,
        ErrorCode::InvalidCursor
    );
}

#[test]
fn legal_call_candidates_are_exact_and_paginated() {
    let (mut workspace, ids) = fixture();
    let module = ids[1];
    let hole = ids[8];
    let transaction = Transaction {
        workspace: workspace.id(),
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: (0..70_u32)
            .map(|index| TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(100 + index),
                module: NodeTarget::Existing(module),
                name: format!("callee-{index:02}"),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: None,
            })
            .collect(),
    };
    let prepared = workspace
        .prepare_transaction(&ApplyTransactionRequest {
            transaction,
            response: TransactionResponseSpec::default(),
        })
        .expect("callee functions");
    workspace
        .publish(prepared.snapshot)
        .expect("publish callees");
    let snapshot = workspace.head().expect("head");
    let target = RepairTarget::Hole(hole);
    let first = legal_constructor_page(
        snapshot,
        target,
        SemanticType::I64,
        PageRequest {
            after: None,
            limit: 64,
        },
    )
    .expect("first constructor page");
    assert_eq!(first.total, Some(77));
    assert_eq!(first.items.len(), 64);
    let second = legal_constructor_page(
        snapshot,
        target,
        SemanticType::I64,
        PageRequest {
            after: first.next,
            limit: 64,
        },
    )
    .expect("second constructor page");
    assert_eq!(second.items.len(), 13);
    assert!(second.next.is_none());
    let mut all = first.items;
    all.extend(second.items);
    assert_eq!(
        all.iter()
            .filter(|constructor| constructor.code == OperationCode::Call)
            .count(),
        71
    );
    let call_targets = all
        .iter()
        .filter_map(|constructor| constructor.call_target)
        .collect::<Vec<_>>();
    assert!(call_targets.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn structured_repair_context_exposes_region_and_loop_argument_roles() {
    let workspace_id = WorkspaceId::from_bytes([0x68; 16]);
    let workspace = Workspace::new(workspace_id).expect("workspace");
    let local = |symbol| NodeTarget::Draft(DraftSymbol::generated(symbol));
    let result = |symbol| ValueDraft::OperationResult {
        operation: local(symbol),
        output: 0,
    };
    let request = ApplyTransactionRequest {
        transaction: Transaction {
            workspace: workspace_id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: local(2),
                    name: "main".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(6)),
                                operation: ExpressionKindDraft::ConstI64(0),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(7)),
                                operation: ExpressionKindDraft::ConstI64(10),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(8)),
                                operation: ExpressionKindDraft::ConstBool(true),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(9)),
                                operation: ExpressionKindDraft::ForI64 {
                                    start: result(6),
                                    end_exclusive: result(7),
                                    step: 1,
                                    initial: result(6),
                                    carried: SemanticType::I64.into(),
                                    index_symbol: DraftSymbol::generated(10),
                                    carried_symbol: DraftSymbol::generated(11),
                                    body: YieldingBodyDraft {
                                        operations: vec![ExpressionDraft {
                                            symbol: Some(DraftSymbol::generated(12)),
                                            operation: ExpressionKindDraft::If {
                                                condition: result(8),
                                                result: SemanticType::I64.into(),
                                                then_body: YieldingBodyDraft {
                                                    operations: vec![ExpressionDraft {
                                                        symbol: Some(DraftSymbol::generated(13)),
                                                        operation: ExpressionKindDraft::Hole {
                                                            expected: SemanticType::I64.into(),
                                                        },
                                                    }],
                                                    yield_value: result(13),
                                                },
                                                else_body: YieldingBodyDraft {
                                                    operations: vec![ExpressionDraft {
                                                        symbol: Some(DraftSymbol::generated(14)),
                                                        operation: ExpressionKindDraft::ConstI64(0),
                                                    }],
                                                    yield_value: result(14),
                                                },
                                            },
                                        }],
                                        yield_value: result(12),
                                    },
                                },
                            },
                        ],
                        return_value: result(9),
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: local(1),
                    function: local(3),
                },
            ],
        },
        response: TransactionResponseSpec {
            return_symbols: [3, 9, 10, 11, 12, 13, 14]
                .into_iter()
                .map(DraftSymbol::generated)
                .collect(),
        },
    };
    let prepared = workspace
        .prepare_transaction(&request)
        .expect("structured query fixture");
    let binding = |symbol| {
        prepared
            .receipt
            .returned_bindings
            .iter()
            .find_map(|(candidate, id)| {
                (*candidate == DraftSymbol::generated(symbol)).then_some(*id)
            })
            .expect("binding")
    };
    let context = repair_context(
        &prepared.snapshot,
        RepairTarget::Hole(binding(13)),
        ContextBudget {
            body_before: 2,
            body_after: 2,
            visible_values: 8,
            incoming_uses: 8,
            include_incompatible: true,
        },
    )
    .expect("structured context");
    assert_eq!(
        context
            .enclosing_regions
            .iter()
            .map(|fact| fact.role)
            .collect::<Vec<_>>(),
        vec![RegionRole::IfThen, RegionRole::ForBody]
    );
    assert_eq!(
        context
            .visible_block_arguments
            .iter()
            .map(|fact| fact.role)
            .collect::<Vec<_>>(),
        vec![BlockArgumentRole::LoopIndex, BlockArgumentRole::LoopCarried]
    );
    assert_eq!(
        context
            .visible_block_arguments
            .iter()
            .map(|fact| fact.argument)
            .collect::<Vec<_>>(),
        vec![binding(10), binding(11)]
    );
    assert!(context.visible_block_arguments.iter().all(|fact| {
        fact.region == context.enclosing_regions[1].region && fact.block != context.owner_block
    }));
    let for_item = body_item(&prepared.snapshot, binding(9), 2, false).expect("for body item");
    assert_eq!(for_item.owned_regions.len(), 1);
    assert_eq!(for_item.owned_regions[0].role, RegionRole::ForBody);
    let call = context
        .legal_constructors
        .iter()
        .find(|constructor| constructor.code == OperationCode::Call)
        .expect("call candidate");
    assert_eq!(call.call_target, Some(binding(3)));
    assert!(call.direct_refinement);
    assert!(
        !context
            .legal_constructors
            .iter()
            .find(|constructor| constructor.code == OperationCode::If)
            .expect("if candidate")
            .direct_refinement
    );
    assert!(
        !context
            .legal_constructors
            .iter()
            .find(|constructor| constructor.code == OperationCode::ForI64)
            .expect("for candidate")
            .direct_refinement
    );
}

#[test]
fn thousands_of_incoming_uses_are_paged_deterministically_without_body_rescans() {
    let workspace_id = WorkspaceId::from_bytes([0x67; 16]);
    let mut workspace = Workspace::new(workspace_id).expect("workspace");
    let local = |value| NodeTarget::Draft(DraftSymbol::generated(value));
    let value = ValueDraft::OperationResult {
        operation: local(6),
        output: 0,
    };
    let mut body_operations = vec![ExpressionDraft {
        symbol: Some(DraftSymbol::generated(6)),
        operation: ExpressionKindDraft::ConstI64(1),
    }];
    for symbol in 100..2100 {
        body_operations.push(ExpressionDraft {
            symbol: Some(DraftSymbol::generated(symbol)),
            operation: ExpressionKindDraft::AddI64 {
                lhs: value.clone(),
                rhs: value.clone(),
            },
        });
    }
    let operations = vec![
        TransactionOp::CreatePackage {
            symbol: DraftSymbol::generated(1),
            name: "app".into(),
        },
        TransactionOp::CreateModule {
            symbol: DraftSymbol::generated(2),
            package: local(1),
            name: "root".into(),
        },
        TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(3),
            module: local(2),
            name: "main".into(),
            parameters: Vec::new(),
            result: SemanticType::I64.into(),
            body: Some(FunctionBodyDraft {
                operations: body_operations,
                return_value: value.clone(),
            }),
        },
        TransactionOp::SetEntryFunction {
            package: local(1),
            function: local(3),
        },
    ];
    let request = ApplyTransactionRequest {
        transaction: Transaction {
            workspace: workspace_id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations,
        },
        response: TransactionResponseSpec {
            return_symbols: vec![DraftSymbol::generated(6)],
        },
    };
    let prepared = workspace
        .prepare_transaction(&request)
        .expect("large deterministic graph");
    let constant = prepared.receipt.returned_bindings[0].1;
    workspace.publish(prepared.snapshot).expect("publish");
    let snapshot = workspace.head().expect("head");
    let value = ValueRef::OperationResult {
        operation: constant,
        output: 0,
    };
    let mut page = uses_page(
        snapshot,
        value,
        PageRequest {
            after: None,
            limit: 64,
        },
    )
    .expect("first uses");
    assert_eq!(page.total, Some(4001));
    let mut seen = page.items.len();
    assert_eq!(
        (page.items[0].operand_index, page.items[1].operand_index),
        (0, 1)
    );
    while let Some(cursor) = page.next {
        page = uses_page(
            snapshot,
            value,
            PageRequest {
                after: Some(cursor),
                limit: 64,
            },
        )
        .expect("continued uses");
        seen += page.items.len();
    }
    assert_eq!(seen, 4001);
}

#[test]
fn batch_policies_partial_outcomes_and_diff_pages_are_deterministic() {
    let (mut workspace, ids) = fixture();
    let id = workspace.id();
    let hole = ids[8];
    let forty = ids[5];
    let two = ids[6];
    let tx = Transaction {
        workspace: id,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::RefineHole {
            hole: NodeTarget::Existing(hole),
            replacement: OperationDraft::AddI64 {
                lhs: ValueDraft::OperationResult {
                    operation: NodeTarget::Existing(forty),
                    output: 0,
                },
                rhs: ValueDraft::OperationResult {
                    operation: NodeTarget::Existing(two),
                    output: 0,
                },
            },
        }],
    };
    let req = ApplyTransactionRequest {
        transaction: tx,
        response: TransactionResponseSpec::default(),
    };
    let prepared = workspace.prepare_transaction(&req).expect("refine");
    let receipt = prepared.receipt.clone();
    workspace.publish(prepared.snapshot).expect("publish");
    let before = workspace.snapshot(Revision::new(1)).expect("before");
    let after = workspace.snapshot(Revision::new(2)).expect("after");
    let first_dependency = dependency_page(
        after,
        hole,
        PageRequest {
            after: None,
            limit: 1,
        },
    )
    .expect("first dependency");
    assert_eq!(first_dependency.items.len(), 1);
    let second_dependency = dependency_page(
        after,
        hole,
        PageRequest {
            after: first_dependency.next,
            limit: 1,
        },
    )
    .expect("continued dependency");
    assert_eq!(second_dependency.items.len(), 1);
    assert!(second_dependency.next.is_none());
    let first = diff_page(
        before,
        after,
        PageRequest {
            after: None,
            limit: 1,
        },
    )
    .expect("diff");
    assert_eq!(first.change_count, receipt.change_count);
    assert_eq!(first.change_digest, receipt.change_digest);
    let mut count = first.page.items.len();
    let mut cursor = first.page.next;
    while let Some(c) = cursor {
        let p = diff_page(
            before,
            after,
            PageRequest {
                after: Some(c),
                limit: 1,
            },
        )
        .expect("next");
        count += p.page.items.len();
        cursor = p.page.next;
    }
    assert_eq!(count as u64, receipt.change_count);
    let duplicate = QueryBatchRequest {
        workspace: id,
        revision: Revision::new(2),
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
    };
    assert_eq!(
        validate_batch(&duplicate).expect_err("duplicate").code,
        ErrorCode::InvalidQuery
    );
    let summaries = |count: usize| QueryBatchRequest {
        workspace: id,
        revision: Revision::new(2),
        queries: (0..count)
            .map(|i| QueryItem {
                id: QueryId::new(i as u64),
                query: Query::WorkspaceSummary,
            })
            .collect(),
    };
    assert_eq!(
        validate_batch(&summaries(33)).expect_err("33 queries").code,
        ErrorCode::PolicyExceeded
    );
    let legal_edge = QueryBatchRequest {
        workspace: id,
        revision: Revision::new(2),
        queries: (0..32)
            .map(|i| QueryItem {
                id: QueryId::new(i),
                query: Query::Body {
                    block: ids[4],
                    page: PageRequest {
                        after: None,
                        limit: 64,
                    },
                },
            })
            .collect(),
    };
    validate_batch(&legal_edge).expect("32 by 64 aggregate edge");
    let aggregate = QueryBatchRequest {
        workspace: id,
        revision: Revision::new(2),
        queries: (0..8)
            .map(|i| QueryItem {
                id: QueryId::new(i),
                query: Query::Body {
                    block: ids[4],
                    page: PageRequest {
                        after: None,
                        limit: 256,
                    },
                },
            })
            .chain(std::iter::once(QueryItem {
                id: QueryId::new(8),
                query: Query::WorkspaceSummary,
            }))
            .collect(),
    };
    assert_eq!(
        validate_batch(&aggregate).expect_err("aggregate 2049").code,
        ErrorCode::PolicyExceeded
    );
    let oversized_page = QueryBatchRequest {
        workspace: id,
        revision: Revision::new(2),
        queries: vec![QueryItem {
            id: QueryId::new(1),
            query: Query::Body {
                block: ids[4],
                page: PageRequest {
                    after: None,
                    limit: 257,
                },
            },
        }],
    };
    assert_eq!(
        validate_batch(&oversized_page).expect_err("page 257").code,
        ErrorCode::PolicyExceeded
    );
    let oversized_context = QueryBatchRequest {
        workspace: id,
        revision: Revision::new(2),
        queries: vec![QueryItem {
            id: QueryId::new(1),
            query: Query::RepairContext {
                target: RepairTarget::Operand {
                    operation: ids[9],
                    index: 0,
                },
                budget: ContextBudget {
                    body_before: 65,
                    body_after: 0,
                    visible_values: 0,
                    incoming_uses: 0,
                    include_incompatible: false,
                },
            },
        }],
    };
    assert_eq!(
        validate_batch(&oversized_context)
            .expect_err("context 65")
            .code,
        ErrorCode::PolicyExceeded
    );
    let ok = execute(after, &Query::WorkspaceSummary, None).expect("success");
    assert!(matches!(ok, QueryResult::WorkspaceSummary(_)));
    let bad = execute(
        after,
        &Query::Body {
            block: hole,
            page: PageRequest {
                after: None,
                limit: 1,
            },
        },
        None,
    )
    .expect_err("item error");
    assert_eq!(bad.code, ErrorCode::WrongKind);
}

fn publish_operations(workspace: &mut Workspace, operations: Vec<TransactionOp>) {
    let request = ApplyTransactionRequest {
        transaction: Transaction {
            workspace: workspace.id(),
            base_revision: workspace.head_revision(),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations,
        },
        response: TransactionResponseSpec::default(),
    };
    let prepared = workspace
        .prepare_transaction(&request)
        .expect("prepare workload");
    workspace
        .publish(prepared.snapshot)
        .expect("publish workload");
}

fn sample_query<F>(mut query: F) -> (u128, u128, usize)
where
    F: FnMut() -> QueryResult,
{
    let warmup = query();
    std::hint::black_box(&warmup);
    let mut samples = Vec::with_capacity(31);
    let mut last = warmup;
    for _ in 0..31 {
        let started = std::time::Instant::now();
        last = query();
        samples.push(started.elapsed().as_nanos());
        std::hint::black_box(&last);
    }
    samples.sort_unstable();
    let bytes = serde_json::to_vec(&last)
        .expect("measure result bytes")
        .len();
    (samples[15], samples[29], bytes)
}

fn sample_batch<F>(mut query: F) -> (u128, u128, usize)
where
    F: FnMut() -> Vec<QueryResult>,
{
    let warmup = query();
    std::hint::black_box(&warmup);
    let mut samples = Vec::with_capacity(31);
    let mut last = warmup;
    for _ in 0..31 {
        let started = std::time::Instant::now();
        last = query();
        samples.push(started.elapsed().as_nanos());
        std::hint::black_box(&last);
    }
    samples.sort_unstable();
    let bytes = serde_json::to_vec(&last)
        .expect("measure batch bytes")
        .len();
    (samples[15], samples[29], bytes)
}

#[test]
#[ignore = "manual scan-based query performance measurement"]
fn query_performance_measurement() {
    let (mut scalar_workspace, scalar_ids) = fixture();
    let scalar_initial = scalar_workspace
        .snapshot(Revision::INITIAL)
        .expect("scalar initial")
        .clone();
    let scalar_before = scalar_workspace
        .snapshot(Revision::new(1))
        .expect("scalar before")
        .clone();
    publish_operations(
        &mut scalar_workspace,
        vec![TransactionOp::RenameNode {
            node: NodeTarget::Existing(scalar_ids[1]),
            name: "renamed".to_owned(),
        }],
    );
    let scalar_after = scalar_workspace.head().expect("scalar after");
    let scalar_block = scalar_ids[4];
    let scalar_hole = scalar_ids[8];
    let context_budget = ContextBudget {
        body_before: 8,
        body_after: 8,
        visible_values: 16,
        incoming_uses: 16,
        include_incompatible: true,
    };
    let summary =
        sample_query(|| execute(scalar_after, &Query::WorkspaceSummary, None).expect("summary"));
    let body = sample_query(|| {
        execute(
            scalar_after,
            &Query::Body {
                block: scalar_block,
                page: PageRequest {
                    after: None,
                    limit: 256,
                },
            },
            None,
        )
        .expect("body")
    });
    let uses = sample_query(|| {
        execute(
            scalar_after,
            &Query::IncomingUses {
                value: ValueRef::OperationResult {
                    operation: scalar_hole,
                    output: 0,
                },
                page: PageRequest {
                    after: None,
                    limit: 256,
                },
            },
            None,
        )
        .expect("uses")
    });
    let context = sample_query(|| {
        execute(
            scalar_after,
            &Query::RepairContext {
                target: RepairTarget::Hole(scalar_hole),
                budget: context_budget,
            },
            None,
        )
        .expect("context")
    });
    let adjacent_diff = sample_query(|| {
        execute(
            scalar_after,
            &Query::SemanticDiff {
                from: Revision::new(1),
                page: PageRequest {
                    after: None,
                    limit: 256,
                },
            },
            Some(&scalar_before),
        )
        .expect("adjacent diff")
    });
    let non_adjacent_diff = sample_query(|| {
        execute(
            scalar_after,
            &Query::SemanticDiff {
                from: Revision::INITIAL,
                page: PageRequest {
                    after: None,
                    limit: 256,
                },
            },
            Some(&scalar_initial),
        )
        .expect("non-adjacent diff")
    });
    let batch_request = QueryBatchRequest {
        workspace: scalar_after.workspace(),
        revision: scalar_after.revision(),
        queries: vec![
            QueryItem {
                id: QueryId::new(1),
                query: Query::WorkspaceSummary,
            },
            QueryItem {
                id: QueryId::new(2),
                query: Query::Body {
                    block: scalar_block,
                    page: PageRequest {
                        after: None,
                        limit: 32,
                    },
                },
            },
            QueryItem {
                id: QueryId::new(3),
                query: Query::IncomingUses {
                    value: ValueRef::OperationResult {
                        operation: scalar_hole,
                        output: 0,
                    },
                    page: PageRequest {
                        after: None,
                        limit: 32,
                    },
                },
            },
            QueryItem {
                id: QueryId::new(4),
                query: Query::RepairContext {
                    target: RepairTarget::Hole(scalar_hole),
                    budget: context_budget,
                },
            },
        ],
    };
    let batch = sample_batch(|| {
        validate_batch(&batch_request).expect("valid measured batch");
        batch_request
            .queries
            .iter()
            .map(|item| execute(scalar_after, &item.query, None).expect("batch item"))
            .collect()
    });

    let (mut body_workspace, body_ids) = fixture();
    let body_block = body_ids[4];
    let body_hole = body_ids[8];
    let body_operations = (0..3_000_u32)
        .map(|index| TransactionOp::InsertExpression {
            block: body_block,
            before: Some(body_hole),
            expression: ExpressionDraft {
                symbol: Some(DraftSymbol::generated(10_000 + index)),
                operation: ExpressionKindDraft::ConstI64(i64::from(index)),
            },
        })
        .collect();
    publish_operations(&mut body_workspace, body_operations);
    let body_snapshot = body_workspace.head().expect("large body");
    let large_body = sample_query(|| {
        execute(
            body_snapshot,
            &Query::Body {
                block: body_block,
                page: PageRequest {
                    after: None,
                    limit: 256,
                },
            },
            None,
        )
        .expect("large body page")
    });
    let large_body_context = sample_query(|| {
        execute(
            body_snapshot,
            &Query::RepairContext {
                target: RepairTarget::Hole(body_hole),
                budget: context_budget,
            },
            None,
        )
        .expect("large body context")
    });

    let (mut unrelated_workspace, unrelated_ids) = fixture();
    let unrelated_hole = unrelated_ids[8];
    let unrelated_operations = (0..3_000_u32)
        .map(|index| TransactionOp::CreatePackage {
            symbol: DraftSymbol::generated(20_000 + index),
            name: format!("unrelated-{index:04}"),
        })
        .collect();
    publish_operations(&mut unrelated_workspace, unrelated_operations);
    let unrelated_snapshot = unrelated_workspace.head().expect("unrelated graph");
    let unrelated_context = sample_query(|| {
        execute(
            unrelated_snapshot,
            &Query::RepairContext {
                target: RepairTarget::Hole(unrelated_hole),
                budget: context_budget,
            },
            None,
        )
        .expect("unrelated context")
    });
    let unrelated_uses = sample_query(|| {
        execute(
            unrelated_snapshot,
            &Query::IncomingUses {
                value: ValueRef::OperationResult {
                    operation: unrelated_hole,
                    output: 0,
                },
                page: PageRequest {
                    after: None,
                    limit: 256,
                },
            },
            None,
        )
        .expect("unrelated uses")
    });

    let measurement = |value: (u128, u128, usize)| {
        serde_json::json!({
            "median_ns": value.0,
            "p95_ns": value.1,
            "json_result_bytes": value.2,
            "samples": 31,
        })
    };
    println!(
        "QUERY_PERFORMANCE {}",
        serde_json::json!({
            "implementation": "full_scans_no_index_or_cache",
            "scalar": {
                "nodes": scalar_after.node_count(),
                "workspace_summary": measurement(summary),
                "body": measurement(body),
                "incoming_uses": measurement(uses),
                "repair_context": measurement(context),
                "adjacent_diff": measurement(adjacent_diff),
                "non_adjacent_diff_0_to_2": measurement(non_adjacent_diff),
                "four_item_batch": measurement(batch),
            },
            "large_body": {
                "nodes": body_snapshot.node_count(),
                "operations_added": 3000,
                "body_first_256": measurement(large_body),
                "repair_context": measurement(large_body_context),
            },
            "unrelated_graph": {
                "nodes": unrelated_snapshot.node_count(),
                "unrelated_packages_added": 3000,
                "repair_context": measurement(unrelated_context),
                "incoming_uses": measurement(unrelated_uses),
            },
        })
    );
}

#[test]
fn nominal_query_names_unrepresentable_member_facts_and_binds_cursor() {
    let id = WorkspaceId::from_bytes([0xb4; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let local = |value| NodeTarget::Draft(DraftSymbol::generated(value));
    let mut operations = vec![
        TransactionOp::CreatePackage {
            symbol: DraftSymbol::generated(1),
            name: "p".into(),
        },
        TransactionOp::CreateModule {
            symbol: DraftSymbol::generated(2),
            package: local(1),
            name: "m".into(),
        },
    ];
    let mut previous = None;
    for index in 0..70_u32 {
        let declaration = DraftSymbol::generated(10 + index * 3);
        let first = DraftSymbol::generated(11 + index * 3);
        let second = DraftSymbol::generated(12 + index * 3);
        let ty = previous.map_or(TypeDraft::I64, |prior| {
            TypeDraft::Nominal(NodeTarget::Draft(prior))
        });
        operations.push(TransactionOp::CreateProductType {
            symbol: declaration,
            module: local(2),
            name: format!("Level{index}"),
            fields: vec![
                ProductFieldDraft {
                    symbol: first,
                    name: "left".into(),
                    ty,
                },
                ProductFieldDraft {
                    symbol: second,
                    name: "right".into(),
                    ty,
                },
            ],
        });
        previous = Some(declaration);
    }
    let prepared = workspace
        .prepare_transaction(&ApplyTransactionRequest {
            transaction: Transaction {
                workspace: id,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::ValidateOnly,
                operations,
            },
            response: TransactionResponseSpec::default(),
        })
        .expect("overflow layouts remain valid graph state");
    let layouts = crate::type_layout::derive_layouts(&prepared.snapshot).expect("layouts");
    let declaration = layouts
        .iter()
        .find_map(|(declaration, layout)| {
            matches!(
                layout,
                DerivedLayout::Unrepresentable(crate::type_layout::LayoutFailure::ByteSizeOverflow)
            )
            .then_some(*declaration)
        })
        .expect("byte overflow declaration");
    let first = nominal_type_result(
        &prepared.snapshot,
        declaration,
        PageRequest {
            after: None,
            limit: 1,
        },
    )
    .expect("first page");
    assert!(first.name.starts_with("Level"));
    assert!(!first.layout.representable);
    assert_eq!(
        first.layout.failure,
        Some(crate::type_layout::LayoutFailure::ByteSizeOverflow)
    );
    let NominalMemberFact::ProductField {
        name,
        offset,
        cells,
        ..
    } = &first.members.items[0]
    else {
        panic!("product field")
    };
    assert_eq!(name, "left");
    assert_eq!((*offset, *cells), (None, None));
    let cursor = first.members.next.expect("cursor");
    let second = nominal_type_result(
        &prepared.snapshot,
        declaration,
        PageRequest {
            after: Some(cursor),
            limit: 1,
        },
    )
    .expect("second page");
    assert_eq!(second.members.items.len(), 1);
    assert!(second.members.next.is_none());
    let other = prepared
        .snapshot
        .nodes()
        .find_map(|(id, node)| {
            (id != declaration && matches!(node, Node::ProductType { .. })).then_some(id)
        })
        .expect("other declaration");
    assert_eq!(
        nominal_type_result(
            &prepared.snapshot,
            other,
            PageRequest {
                after: Some(cursor),
                limit: 1,
            },
        )
        .expect_err("cross declaration cursor")
        .code,
        ErrorCode::InvalidCursor
    );
}
