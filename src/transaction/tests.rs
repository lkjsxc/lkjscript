use super::*;
use crate::artifact;

fn request(transaction: &Transaction) -> ApplyTransactionRequest {
    let mut return_symbols: Vec<DraftSymbol> = scan_explicit_symbols(&transaction.operations)
        .expect("valid test symbols")
        .into_iter()
        .collect();
    return_symbols.sort();
    return_symbols.truncate(MAX_RETURNED_BINDINGS);
    ApplyTransactionRequest {
        transaction: transaction.clone(),
        response: TransactionResponseSpec { return_symbols },
    }
}

fn commit(workspace: &mut Workspace, transaction: &Transaction) -> Result<TransactionReceipt> {
    let prepared = workspace.prepare_transaction(&request(transaction))?;
    let receipt = prepared.receipt.clone();
    if transaction.mode == TransactionMode::Commit {
        workspace.publish(prepared.snapshot)?;
    }
    Ok(receipt)
}

fn create_package_and_module(id: WorkspaceId) -> Transaction {
    Transaction {
        workspace: id,
        base_revision: Revision::INITIAL,
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![
            TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(1),
                name: "package".to_owned(),
            },
            TransactionOp::CreateModule {
                symbol: DraftSymbol::generated(2),
                package: NodeTarget::Draft(DraftSymbol::generated(1)),
                name: "module".to_owned(),
            },
        ],
    }
}

fn draft_symbol(value: u32) -> NodeTarget {
    NodeTarget::Draft(DraftSymbol::generated(value))
}
fn draft_result(value: u32) -> ValueDraft {
    ValueDraft::OperationResult {
        operation: draft_symbol(value),
        output: 0,
    }
}
fn draft_expression(symbol: u32, operation: ExpressionKindDraft) -> ExpressionDraft {
    ExpressionDraft {
        symbol: Some(DraftSymbol::generated(symbol)),
        operation,
    }
}
fn inline(operation: ExpressionKindDraft) -> ValueDraft {
    ValueDraft::InlineExpression(Box::new(operation))
}
fn structured_semantic_request(
    id: WorkspaceId,
    mut operations: Vec<TransactionOp>,
) -> ApplyTransactionRequest {
    let mut all = vec![
        TransactionOp::CreatePackage {
            symbol: DraftSymbol::generated(1),
            name: "package".into(),
        },
        TransactionOp::CreateModule {
            symbol: DraftSymbol::generated(2),
            package: draft_symbol(1),
            name: "module".into(),
        },
    ];
    all.append(&mut operations);
    ApplyTransactionRequest {
        transaction: Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: all,
        },
        response: TransactionResponseSpec::default(),
    }
}

fn equal_arithmetic_request(id: WorkspaceId, inline_values: bool) -> ApplyTransactionRequest {
    let operations = if inline_values {
        vec![draft_expression(
            8,
            ExpressionKindDraft::AddI64 {
                lhs: inline(ExpressionKindDraft::AddI64 {
                    lhs: inline(ExpressionKindDraft::ConstI64(1)),
                    rhs: inline(ExpressionKindDraft::ConstI64(2)),
                }),
                rhs: inline(ExpressionKindDraft::ConstI64(3)),
            },
        )]
    } else {
        vec![
            draft_expression(4, ExpressionKindDraft::ConstI64(1)),
            draft_expression(5, ExpressionKindDraft::ConstI64(2)),
            draft_expression(
                6,
                ExpressionKindDraft::AddI64 {
                    lhs: draft_result(4),
                    rhs: draft_result(5),
                },
            ),
            draft_expression(7, ExpressionKindDraft::ConstI64(3)),
            draft_expression(
                8,
                ExpressionKindDraft::AddI64 {
                    lhs: draft_result(6),
                    rhs: draft_result(7),
                },
            ),
        ]
    };
    let mut request = structured_semantic_request(
        id,
        vec![
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "main".into(),
                parameters: Vec::new(),
                result: TypeDraft::I64,
                body: Some(FunctionBodyDraft {
                    operations,
                    return_value: draft_result(8),
                }),
            },
            TransactionOp::SetEntryFunction {
                package: draft_symbol(1),
                function: draft_symbol(3),
            },
        ],
    );
    request.response.return_symbols = [1, 2, 3, 8]
        .into_iter()
        .map(DraftSymbol::generated)
        .collect();
    request
}

#[test]
fn inline_and_explicit_postorder_proposals_produce_identical_authority() {
    let id = WorkspaceId::from_bytes([0x70; 16]);
    let explicit_workspace = Workspace::new(id).expect("explicit workspace");
    let inline_workspace = Workspace::new(id).expect("inline workspace");
    let explicit = explicit_workspace
        .prepare_transaction(&equal_arithmetic_request(id, false))
        .expect("explicit proposal");
    let inline = inline_workspace
        .prepare_transaction(&equal_arithmetic_request(id, true))
        .expect("inline proposal");

    assert_eq!(explicit.receipt, inline.receipt);
    assert_eq!(explicit.snapshot.hash(), inline.snapshot.hash());
    assert_eq!(
        explicit.snapshot.nodes().collect::<Vec<_>>(),
        inline.snapshot.nodes().collect::<Vec<_>>()
    );
    assert_eq!(
        artifact::encode(&explicit.snapshot).expect("explicit artifact"),
        artifact::encode(&inline.snapshot).expect("inline artifact")
    );
}

fn equal_named_call_request(id: WorkspaceId, inline_values: bool) -> ApplyTransactionRequest {
    let value_tree = || {
        inline(ExpressionKindDraft::ConstructProduct {
            product: draft_symbol(3),
            fields: vec![ProductFieldValueDraft {
                field: draft_symbol(4),
                value: inline(ExpressionKindDraft::ProjectField {
                    value: inline(ExpressionKindDraft::Call {
                        function: draft_symbol(7),
                        arguments: vec![inline(ExpressionKindDraft::ConstructProduct {
                            product: draft_symbol(3),
                            fields: vec![ProductFieldValueDraft {
                                field: draft_symbol(4),
                                value: inline(ExpressionKindDraft::ConstI64(9)),
                            }],
                        })],
                    }),
                    field: draft_symbol(4),
                }),
            }],
        })
    };
    let operations = if inline_values {
        vec![draft_expression(
            15,
            ExpressionKindDraft::ConstructVariant {
                variant: draft_symbol(6),
                payload: Some(value_tree()),
            },
        )]
    } else {
        vec![
            draft_expression(10, ExpressionKindDraft::ConstI64(9)),
            draft_expression(
                11,
                ExpressionKindDraft::ConstructProduct {
                    product: draft_symbol(3),
                    fields: vec![ProductFieldValueDraft {
                        field: draft_symbol(4),
                        value: draft_result(10),
                    }],
                },
            ),
            draft_expression(
                12,
                ExpressionKindDraft::Call {
                    function: draft_symbol(7),
                    arguments: vec![draft_result(11)],
                },
            ),
            draft_expression(
                13,
                ExpressionKindDraft::ProjectField {
                    value: draft_result(12),
                    field: draft_symbol(4),
                },
            ),
            draft_expression(
                14,
                ExpressionKindDraft::ConstructProduct {
                    product: draft_symbol(3),
                    fields: vec![ProductFieldValueDraft {
                        field: draft_symbol(4),
                        value: draft_result(13),
                    }],
                },
            ),
            draft_expression(
                15,
                ExpressionKindDraft::ConstructVariant {
                    variant: draft_symbol(6),
                    payload: Some(draft_result(14)),
                },
            ),
        ]
    };
    let mut request = structured_semantic_request(
        id,
        vec![
            TransactionOp::CreateProductType {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "BoxedI64".into(),
                fields: vec![ProductFieldDraft {
                    symbol: DraftSymbol::generated(4),
                    name: "value".into(),
                    ty: TypeDraft::I64,
                }],
            },
            TransactionOp::CreateSumType {
                symbol: DraftSymbol::generated(5),
                module: draft_symbol(2),
                name: "MaybeBox".into(),
                variants: vec![SumVariantDraft {
                    symbol: DraftSymbol::generated(6),
                    name: "some".into(),
                    payload: Some(TypeDraft::Nominal(draft_symbol(3))),
                }],
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(9),
                module: draft_symbol(2),
                name: "main".into(),
                parameters: Vec::new(),
                result: TypeDraft::Nominal(draft_symbol(5)),
                body: Some(FunctionBodyDraft {
                    operations,
                    return_value: draft_result(15),
                }),
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(7),
                module: draft_symbol(2),
                name: "identity".into(),
                parameters: vec![FunctionParameterDraft {
                    symbol: DraftSymbol::generated(8),
                    name: "value".into(),
                    ty: TypeDraft::Nominal(draft_symbol(3)),
                }],
                result: TypeDraft::Nominal(draft_symbol(3)),
                body: Some(FunctionBodyDraft {
                    operations: Vec::new(),
                    return_value: ValueDraft::FunctionParameter(draft_symbol(8)),
                }),
            },
            TransactionOp::SetEntryFunction {
                package: draft_symbol(1),
                function: draft_symbol(9),
            },
        ],
    );
    request.response.return_symbols = [1, 2, 3, 4, 5, 6, 7, 8, 9, 15]
        .into_iter()
        .map(DraftSymbol::generated)
        .collect();
    request
}

#[test]
fn inline_calls_products_projections_and_variants_are_byte_identical() {
    let id = WorkspaceId::from_bytes([0x74; 16]);
    let explicit_workspace = Workspace::new(id).expect("explicit workspace");
    let inline_workspace = Workspace::new(id).expect("inline workspace");
    let explicit = explicit_workspace
        .prepare_transaction(&equal_named_call_request(id, false))
        .expect("explicit proposal");
    let inline = inline_workspace
        .prepare_transaction(&equal_named_call_request(id, true))
        .expect("inline proposal");
    assert_eq!(explicit.receipt, inline.receipt);
    assert_eq!(
        artifact::encode(&explicit.snapshot).expect("explicit artifact"),
        artifact::encode(&inline.snapshot).expect("inline artifact")
    );
}

fn equal_bytes_request(id: WorkspaceId, inline_values: bool) -> ApplyTransactionRequest {
    let operations = if inline_values {
        vec![draft_expression(
            11,
            ExpressionKindDraft::BytesEqual {
                lhs: inline(ExpressionKindDraft::BytesConcat {
                    lhs: inline(ExpressionKindDraft::BytesSlice {
                        value: inline(ExpressionKindDraft::ConstBytes(
                            ByteString::from_slice(b"LKJMpayload").expect("literal"),
                        )),
                        start: inline(ExpressionKindDraft::ConstI64(0)),
                        length: inline(ExpressionKindDraft::ConstI64(4)),
                    }),
                    rhs: inline(ExpressionKindDraft::ConstBytes(
                        ByteString::from_slice(b"!").expect("literal"),
                    )),
                }),
                rhs: inline(ExpressionKindDraft::ConstBytes(
                    ByteString::from_slice(b"LKJM!").expect("literal"),
                )),
            },
        )]
    } else {
        vec![
            draft_expression(
                4,
                ExpressionKindDraft::ConstBytes(
                    ByteString::from_slice(b"LKJMpayload").expect("literal"),
                ),
            ),
            draft_expression(5, ExpressionKindDraft::ConstI64(0)),
            draft_expression(6, ExpressionKindDraft::ConstI64(4)),
            draft_expression(
                7,
                ExpressionKindDraft::BytesSlice {
                    value: draft_result(4),
                    start: draft_result(5),
                    length: draft_result(6),
                },
            ),
            draft_expression(
                8,
                ExpressionKindDraft::ConstBytes(ByteString::from_slice(b"!").expect("literal")),
            ),
            draft_expression(
                9,
                ExpressionKindDraft::BytesConcat {
                    lhs: draft_result(7),
                    rhs: draft_result(8),
                },
            ),
            draft_expression(
                10,
                ExpressionKindDraft::ConstBytes(ByteString::from_slice(b"LKJM!").expect("literal")),
            ),
            draft_expression(
                11,
                ExpressionKindDraft::BytesEqual {
                    lhs: draft_result(9),
                    rhs: draft_result(10),
                },
            ),
        ]
    };
    let mut request = structured_semantic_request(
        id,
        vec![
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "magic_matches".into(),
                parameters: Vec::new(),
                result: TypeDraft::Bool,
                body: Some(FunctionBodyDraft {
                    operations,
                    return_value: draft_result(11),
                }),
            },
            TransactionOp::SetEntryFunction {
                package: draft_symbol(1),
                function: draft_symbol(3),
            },
        ],
    );
    request.response.return_symbols = [1, 2, 3, 11]
        .into_iter()
        .map(DraftSymbol::generated)
        .collect();
    request
}

#[test]
fn explicit_and_inline_byte_expressions_produce_identical_authority_and_result() {
    let id = WorkspaceId::from_bytes([0xb4; 16]);
    let explicit = Workspace::new(id)
        .expect("explicit workspace")
        .prepare_transaction(&equal_bytes_request(id, false))
        .expect("explicit bytes proposal");
    let inline = Workspace::new(id)
        .expect("inline workspace")
        .prepare_transaction(&equal_bytes_request(id, true))
        .expect("inline bytes proposal");
    assert_eq!(explicit.receipt, inline.receipt);
    assert_eq!(explicit.snapshot.hash(), inline.snapshot.hash());
    assert_eq!(
        artifact::encode(&explicit.snapshot).expect("explicit artifact"),
        artifact::encode(&inline.snapshot).expect("inline artifact")
    );
    let entry = explicit.receipt.returned_bindings[2].1;
    let run = crate::interpret::compile_and_run(
        &explicit.snapshot,
        entry,
        &[],
        crate::interpret::RunPolicy {
            fuel: 100,
            maximum_frames: 16,
        },
    )
    .expect("byte expression execution");
    assert_eq!(run.value, crate::interpret::RuntimeValue::Bool(true));
}

fn literal_budget_request(id: WorkspaceId, lengths: &[usize]) -> ApplyTransactionRequest {
    let operations = lengths
        .iter()
        .enumerate()
        .map(|(index, length)| {
            draft_expression(
                100 + u32::try_from(index).expect("bounded test index"),
                ExpressionKindDraft::ConstBytes(
                    ByteString::new(vec![0xa5; *length]).expect("public byte bound"),
                ),
            )
        })
        .collect::<Vec<_>>();
    structured_semantic_request(
        id,
        vec![TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(3),
            module: draft_symbol(2),
            name: "literal_budget".into(),
            parameters: Vec::new(),
            result: TypeDraft::Bytes,
            body: Some(FunctionBodyDraft {
                operations,
                return_value: draft_result(100),
            }),
        }],
    )
}

#[test]
fn byte_literal_and_transaction_aggregate_limits_reject_before_identity_consumption() {
    let id = WorkspaceId::from_bytes([0xb5; 16]);
    let exact = vec![MAXIMUM_BYTE_LITERAL_BYTES; 16];
    Workspace::new(id)
        .expect("exact workspace")
        .prepare_transaction(&literal_budget_request(id, &exact))
        .expect("exact aggregate byte literal limit");

    for (name, lengths) in [
        ("one literal", vec![MAXIMUM_BYTE_LITERAL_BYTES + 1]),
        (
            "aggregate literals",
            exact.into_iter().chain([1]).collect::<Vec<_>>(),
        ),
    ] {
        let workspace = Workspace::new(id).expect("rejecting workspace");
        let error = workspace
            .prepare_transaction(&literal_budget_request(id, &lengths))
            .expect_err(name);
        assert_eq!(error.code, ErrorCode::ByteLiteralTooLarge);
        assert_eq!(workspace.head_revision(), Revision::INITIAL);
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
    }
}

#[test]
fn byte_hole_rejects_wrong_type_and_refines_without_identity_churn() {
    let id = WorkspaceId::from_bytes([0xb7; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let mut initial = structured_semantic_request(
        id,
        vec![TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(3),
            module: draft_symbol(2),
            name: "repair_bytes".into(),
            parameters: Vec::new(),
            result: TypeDraft::Bytes,
            body: Some(FunctionBodyDraft {
                operations: vec![draft_expression(
                    4,
                    ExpressionKindDraft::Hole {
                        expected: TypeDraft::Bytes,
                    },
                )],
                return_value: draft_result(4),
            }),
        }],
    );
    initial.response.return_symbols = vec![DraftSymbol::generated(4)];
    let prepared = workspace
        .prepare_transaction(&initial)
        .expect("incomplete byte function");
    let hole = prepared.receipt.returned_bindings[0].1;
    workspace.publish(prepared.snapshot).expect("publish hole");
    let next_serial = workspace.head().expect("head").next_serial();

    let refinement = |mode, replacement| ApplyTransactionRequest {
        transaction: Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement,
            }],
        },
        response: TransactionResponseSpec::default(),
    };
    let error = workspace
        .prepare_transaction(&refinement(
            TransactionMode::Commit,
            OperationDraft::ConstI64(1),
        ))
        .expect_err("wrong byte repair type");
    assert_eq!(error.code, ErrorCode::TypeMismatch);
    assert_eq!(error.expected_type, Some(SemanticType::Bytes));
    assert_eq!(error.actual_type, Some(SemanticType::I64));
    assert_eq!(workspace.head_revision(), Revision::new(1));
    assert_eq!(workspace.head().expect("head").next_serial(), next_serial);

    let replacement =
        OperationDraft::ConstBytes(ByteString::from_slice(b"repaired").expect("byte replacement"));
    let predicted = workspace
        .prepare_transaction(&refinement(
            TransactionMode::ValidateOnly,
            replacement.clone(),
        ))
        .expect("validate byte repair");
    assert_eq!(predicted.receipt.created_count, 0);
    assert!(!predicted.receipt.published);
    assert_eq!(workspace.head_revision(), Revision::new(1));
    assert_eq!(workspace.head().expect("head").next_serial(), next_serial);

    let committed = workspace
        .prepare_transaction(&refinement(TransactionMode::Commit, replacement))
        .expect("commit byte repair");
    assert_eq!(committed.receipt.revision, Revision::new(2));
    assert_eq!(committed.receipt.created_count, 0);
    assert_eq!(
        committed.snapshot.node(hole).expect("refined hole").kind(),
        NodeKind::Operation
    );
    let Node::Operation { operation, .. } = committed.snapshot.node(hole).expect("operation")
    else {
        panic!("refined byte hole kind")
    };
    assert!(
        matches!(operation, OperationKind::ConstBytes(value) if value.as_slice() == b"repaired")
    );
    assert_eq!(committed.snapshot.next_serial(), next_serial);
}

#[test]
fn inline_validate_only_and_commit_predict_the_same_ids_without_allocation() {
    let id = WorkspaceId::from_bytes([0x71; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let mut validate = equal_arithmetic_request(id, true);
    validate.transaction.mode = TransactionMode::ValidateOnly;
    let predicted = workspace
        .prepare_transaction(&validate)
        .expect("validate-only proposal");
    assert_eq!(workspace.head_revision(), Revision::INITIAL);
    assert_eq!(workspace.head().expect("head").next_serial(), 2);

    let committed = workspace
        .prepare_transaction(&equal_arithmetic_request(id, true))
        .expect("commit proposal");
    let mut expected = predicted.receipt;
    expected.published = true;
    assert_eq!(committed.receipt, expected);
    assert_eq!(committed.snapshot.hash(), predicted.snapshot.hash());
}

#[test]
fn function_body_replacement_preserves_entity_without_durable_churn() {
    let id = WorkspaceId::from_bytes([0x6f; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let initial = workspace
        .prepare_transaction(&equal_arithmetic_request(id, false))
        .expect("initial function");
    let function = binding(&initial.receipt, 3);
    let old_result = binding(&initial.receipt, 8);
    let next_serial = initial.snapshot.next_serial();
    let tombstones = initial.snapshot.tombstones().collect::<Vec<_>>();
    workspace
        .publish(initial.snapshot)
        .expect("publish initial");

    let replacement_symbol = DraftSymbol::new("answer");
    let replacement = ApplyTransactionRequest {
        transaction: Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::ValidateOnly,
            operations: vec![TransactionOp::ReplaceFunctionBody {
                function,
                body: FunctionBodyDraft {
                    operations: vec![ExpressionDraft {
                        symbol: Some(replacement_symbol),
                        operation: ExpressionKindDraft::ConstI64(42),
                    }],
                    return_value: ValueDraft::OperationResult {
                        operation: NodeTarget::Draft(replacement_symbol),
                        output: 0,
                    },
                },
            }],
        },
        response: TransactionResponseSpec {
            return_symbols: vec![replacement_symbol],
        },
    };
    let predicted = workspace
        .prepare_transaction(&replacement)
        .expect("validate replacement");
    let replacement_result = predicted.receipt.returned_bindings[0].1;
    assert_eq!(predicted.receipt.created_count, 0);
    assert_eq!(predicted.snapshot.next_serial(), next_serial);
    assert_eq!(
        predicted.snapshot.tombstones().collect::<Vec<_>>(),
        tombstones
    );
    assert!(predicted.snapshot.node(function).is_ok());
    assert!(predicted.snapshot.node(old_result).is_err());
    assert!(replacement_result.is_function_local());
    assert_ne!(replacement_result, old_result);
    assert!(
        diff::between(workspace.head().expect("head"), &predicted.snapshot)
            .changes
            .iter()
            .any(|change| change.node == function
                && matches!(
                    change.kind,
                    crate::diff::ChangeKind::FunctionBodyChanged { .. }
                ))
    );

    let mut committed_request = replacement;
    committed_request.transaction.mode = TransactionMode::Commit;
    let committed = workspace
        .prepare_transaction(&committed_request)
        .expect("commit replacement");
    let mut expected = predicted.receipt;
    expected.published = true;
    assert_eq!(committed.receipt, expected);
    assert_eq!(committed.snapshot.hash(), predicted.snapshot.hash());

    let durable_count = committed.snapshot.durable_identity_count();
    let local_count = committed.snapshot.function_local_reference_count();
    workspace
        .publish(committed.snapshot)
        .expect("publish first replacement");
    let mut artifact_sizes = Vec::new();
    for value in 0_i64..32 {
        let symbol = DraftSymbol::new("body_value");
        let request = ApplyTransactionRequest {
            transaction: Transaction {
                workspace: id,
                base_revision: workspace.head_revision(),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::ReplaceFunctionBody {
                    function,
                    body: FunctionBodyDraft {
                        operations: vec![ExpressionDraft {
                            symbol: Some(symbol),
                            operation: ExpressionKindDraft::ConstI64(value),
                        }],
                        return_value: ValueDraft::OperationResult {
                            operation: NodeTarget::Draft(symbol),
                            output: 0,
                        },
                    },
                }],
            },
            response: TransactionResponseSpec::default(),
        };
        let candidate = workspace
            .prepare_transaction(&request)
            .expect("identity-pressure replacement");
        assert_eq!(candidate.receipt.created_count, 0);
        assert_eq!(candidate.snapshot.next_serial(), next_serial);
        assert_eq!(candidate.snapshot.durable_identity_count(), durable_count);
        assert_eq!(
            candidate.snapshot.function_local_reference_count(),
            local_count
        );
        assert_eq!(
            candidate.snapshot.tombstones().collect::<Vec<_>>(),
            tombstones
        );
        artifact_sizes.push(
            artifact::encode(&candidate.snapshot)
                .expect("identity-pressure artifact")
                .len(),
        );
        workspace
            .publish(candidate.snapshot)
            .expect("publish identity-pressure replacement");
    }
    eprintln!(
        "identity_pressure revisions=32 durable={} local={} tombstones={} artifact_min={} artifact_max={}",
        durable_count,
        local_count,
        tombstones.len(),
        artifact_sizes.iter().min().expect("artifact size"),
        artifact_sizes.iter().max().expect("artifact size")
    );
}

#[test]
fn function_body_replacement_rejects_implicit_anchor_deletion() {
    let id = WorkspaceId::from_bytes([0x6e; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let created = commit(&mut workspace, &incomplete_program(id)).expect("anchored body");
    let function = binding(&created, 3);
    let hole = binding(&created, 9);
    assert!(function.is_durable());
    assert!(hole.is_durable());
    let before = workspace.head().expect("head").clone();
    let request = ApplyTransactionRequest {
        transaction: Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::ReplaceFunctionBody {
                function,
                body: FunctionBodyDraft {
                    operations: Vec::new(),
                    return_value: ValueDraft::InlineExpression(Box::new(
                        ExpressionKindDraft::ConstI64(1),
                    )),
                },
            }],
        },
        response: TransactionResponseSpec::default(),
    };
    let error = workspace
        .prepare_transaction(&request)
        .expect_err("anchor deletion must be explicit");
    assert_eq!(error.code, ErrorCode::DeleteBlocked);
    assert_eq!(error.target, Some(hole));
    assert_eq!(workspace.head().expect("head"), &before);
}

#[test]
fn inline_depth_and_eligibility_reject_before_allocation_with_exact_paths() {
    let id = WorkspaceId::from_bytes([0x72; 16]);
    let block = NodeId::new(id, 2).expect("block");
    let operation = |value| TransactionOp::InsertExpression {
        block,
        before: None,
        expression: ExpressionDraft {
            symbol: Some(DraftSymbol::generated(1)),
            operation: ExpressionKindDraft::AddI64 {
                lhs: value,
                rhs: inline(ExpressionKindDraft::ConstI64(0)),
            },
        },
    };
    let nested = |depth: usize| {
        let mut value = inline(ExpressionKindDraft::ConstI64(1));
        for _ in 1..depth {
            value = inline(ExpressionKindDraft::AddI64 {
                lhs: value,
                rhs: inline(ExpressionKindDraft::ConstI64(1)),
            });
        }
        value
    };

    validate_structured_request(&[operation(nested(MAX_STRUCTURED_DRAFT_DEPTH))])
        .expect("maximum accepted mixed inline depth");
    let excessive =
        validate_structured_request(&[operation(nested(MAX_STRUCTURED_DRAFT_DEPTH + 1))])
            .expect_err("first excessive mixed inline depth");
    assert_eq!(excessive.code, ErrorCode::PolicyExceeded);
    assert_eq!(excessive.operation_index, Some(0));
    assert!(excessive.draft_path.is_some());

    for forbidden in [
        ExpressionKindDraft::Hole {
            expected: TypeDraft::I64,
        },
        ExpressionKindDraft::If {
            condition: inline(ExpressionKindDraft::ConstBool(true)),
            result: TypeDraft::I64,
            then_body: YieldingBodyDraft {
                operations: Vec::new(),
                yield_value: inline(ExpressionKindDraft::ConstI64(1)),
            },
            else_body: YieldingBodyDraft {
                operations: Vec::new(),
                yield_value: inline(ExpressionKindDraft::ConstI64(2)),
            },
        },
    ] {
        let error = validate_structured_request(&[operation(inline(forbidden))])
            .expect_err("ineligible inline expression");
        assert_eq!(error.code, ErrorCode::InvalidOperand);
        assert_eq!(error.draft_path.as_deref(), Some("op[0].expression.lhs"));
    }

    let exact_inline_count = (MAX_STRUCTURED_DRAFT_ITEMS - 2) / 2;
    let call_with_inline_arguments = |count| TransactionOp::InsertExpression {
        block,
        before: None,
        expression: ExpressionDraft {
            symbol: Some(DraftSymbol::generated(2)),
            operation: ExpressionKindDraft::Call {
                function: NodeTarget::Existing(block),
                arguments: vec![inline(ExpressionKindDraft::ConstI64(1)); count],
            },
        },
    };
    validate_structured_request(&[call_with_inline_arguments(exact_inline_count)])
        .expect("exact inline item limit");
    assert_eq!(
        validate_structured_request(&[call_with_inline_arguments(exact_inline_count + 1)])
            .expect_err("first excessive inline item")
            .code,
        ErrorCode::PolicyExceeded
    );
}

#[test]
fn invalid_inline_type_reports_anonymous_path_and_rolls_back() {
    let id = WorkspaceId::from_bytes([0x73; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let mut request = equal_arithmetic_request(id, true);
    let TransactionOp::CreateFunction {
        body: Some(body), ..
    } = &mut request.transaction.operations[2]
    else {
        panic!("function body");
    };
    let ExpressionKindDraft::AddI64 { lhs, .. } = &mut body.operations[0].operation else {
        panic!("outer add");
    };
    *lhs = inline(ExpressionKindDraft::ConstBool(true));

    let error = workspace
        .prepare_transaction(&request)
        .expect_err("inline type mismatch");
    assert_eq!(error.code, ErrorCode::TypeMismatch);
    assert_eq!(error.operation_index, Some(2));
    assert_eq!(error.draft_path.as_deref(), Some("op[2].body.e[0].lhs"));
    assert!(error.draft_symbol.is_none());
    assert_eq!(workspace.head_revision(), Revision::INITIAL);
    assert_eq!(workspace.head().expect("head").next_serial(), 2);
}

#[test]
fn maintenance_operations_reject_inline_values_before_allocation() {
    let id = WorkspaceId::from_bytes([0x75; 16]);
    let node = NodeTarget::Existing(NodeId::new(id, 2).expect("node"));
    for operation in [
        TransactionOp::ReplaceOperand {
            operation: node,
            index: 0,
            value: inline(ExpressionKindDraft::ConstI64(1)),
        },
        TransactionOp::ReplaceOperation {
            operation: node,
            replacement: OperationDraft::AddI64 {
                lhs: inline(ExpressionKindDraft::ConstI64(1)),
                rhs: ValueDraft::OperationResult {
                    operation: node,
                    output: 0,
                },
            },
        },
    ] {
        let error = validate_structured_request(&[operation])
            .expect_err("maintenance inline value must reject");
        assert_eq!(error.code, ErrorCode::InvalidOperand);
        assert_eq!(error.operation_index, Some(0));
    }
}

#[test]
fn negative_for_step_rejects_atomically() {
    let id = WorkspaceId::from_bytes([0xa1; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let request = structured_semantic_request(
        id,
        vec![TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(3),
            module: draft_symbol(2),
            name: "negative".into(),
            parameters: Vec::new(),
            result: SemanticType::I64.into(),
            body: Some(FunctionBodyDraft {
                operations: vec![
                    draft_expression(4, ExpressionKindDraft::ConstI64(0)),
                    draft_expression(5, ExpressionKindDraft::ConstI64(2)),
                    draft_expression(
                        6,
                        ExpressionKindDraft::ForI64 {
                            start: draft_result(4),
                            end_exclusive: draft_result(5),
                            step: -1,
                            initial: draft_result(4),
                            carried: SemanticType::I64.into(),
                            index_symbol: DraftSymbol::generated(7),
                            carried_symbol: DraftSymbol::generated(8),
                            body: YieldingBodyDraft {
                                operations: Vec::new(),
                                yield_value: ValueDraft::BlockArgument(draft_symbol(8)),
                            },
                        },
                    ),
                ],
                return_value: draft_result(6),
            }),
        }],
    );
    let error = workspace
        .prepare_transaction(&request)
        .expect_err("negative step");
    assert_eq!(error.code, ErrorCode::InvalidOperand);
    assert_eq!(workspace.head().expect("head").next_serial(), 2);
}

#[test]
fn sibling_if_arm_local_capture_rejects_atomically() {
    let id = WorkspaceId::from_bytes([0xa2; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let request = structured_semantic_request(
        id,
        vec![TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(3),
            module: draft_symbol(2),
            name: "sibling".into(),
            parameters: Vec::new(),
            result: SemanticType::I64.into(),
            body: Some(FunctionBodyDraft {
                operations: vec![
                    draft_expression(4, ExpressionKindDraft::ConstBool(true)),
                    draft_expression(
                        5,
                        ExpressionKindDraft::If {
                            condition: draft_result(4),
                            result: SemanticType::I64.into(),
                            then_body: YieldingBodyDraft {
                                operations: vec![draft_expression(
                                    6,
                                    ExpressionKindDraft::ConstI64(1),
                                )],
                                yield_value: draft_result(6),
                            },
                            else_body: YieldingBodyDraft {
                                operations: Vec::new(),
                                yield_value: draft_result(6),
                            },
                        },
                    ),
                ],
                return_value: draft_result(5),
            }),
        }],
    );
    let error = workspace
        .prepare_transaction(&request)
        .expect_err("sibling capture");
    assert_eq!(error.code, ErrorCode::InvalidOperand);
    assert_eq!(workspace.head().expect("head").next_serial(), 2);
}

#[test]
fn nested_local_escape_after_owning_operation_rejects_atomically() {
    let id = WorkspaceId::from_bytes([0xa3; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let request = structured_semantic_request(
        id,
        vec![TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(3),
            module: draft_symbol(2),
            name: "escape".into(),
            parameters: Vec::new(),
            result: SemanticType::I64.into(),
            body: Some(FunctionBodyDraft {
                operations: vec![
                    draft_expression(4, ExpressionKindDraft::ConstBool(true)),
                    draft_expression(
                        5,
                        ExpressionKindDraft::If {
                            condition: draft_result(4),
                            result: SemanticType::I64.into(),
                            then_body: YieldingBodyDraft {
                                operations: vec![draft_expression(
                                    6,
                                    ExpressionKindDraft::ConstI64(1),
                                )],
                                yield_value: draft_result(6),
                            },
                            else_body: YieldingBodyDraft {
                                operations: vec![draft_expression(
                                    7,
                                    ExpressionKindDraft::ConstI64(2),
                                )],
                                yield_value: draft_result(7),
                            },
                        },
                    ),
                    draft_expression(
                        8,
                        ExpressionKindDraft::AddI64 {
                            lhs: draft_result(6),
                            rhs: draft_result(5),
                        },
                    ),
                ],
                return_value: draft_result(8),
            }),
        }],
    );
    let error = workspace
        .prepare_transaction(&request)
        .expect_err("nested escape");
    assert_eq!(error.code, ErrorCode::InvalidOperand);
    assert_eq!(workspace.head().expect("head").next_serial(), 2);
}

#[test]
fn cross_function_direct_value_use_rejects_atomically() {
    let id = WorkspaceId::from_bytes([0xa4; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let request = structured_semantic_request(
        id,
        vec![
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "producer".into(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: vec![draft_expression(4, ExpressionKindDraft::ConstI64(1))],
                    return_value: draft_result(4),
                }),
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(5),
                module: draft_symbol(2),
                name: "consumer".into(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: Vec::new(),
                    return_value: draft_result(4),
                }),
            },
        ],
    );
    let error = workspace
        .prepare_transaction(&request)
        .expect_err("cross function value");
    assert_eq!(error.code, ErrorCode::InvalidOperand);
    assert_eq!(workspace.head().expect("head").next_serial(), 2);
}

#[test]
fn same_workspace_cross_module_call_succeeds() {
    let id = WorkspaceId::from_bytes([0xa5; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let request = ApplyTransactionRequest {
        transaction: Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "package".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: draft_symbol(1),
                    name: "left".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(3),
                    package: draft_symbol(1),
                    name: "right".into(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(4),
                    module: draft_symbol(2),
                    name: "callee".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![draft_expression(5, ExpressionKindDraft::ConstI64(7))],
                        return_value: draft_result(5),
                    }),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(6),
                    module: draft_symbol(3),
                    name: "caller".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![draft_expression(
                            7,
                            ExpressionKindDraft::Call {
                                function: draft_symbol(4),
                                arguments: Vec::new(),
                            },
                        )],
                        return_value: draft_result(7),
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: draft_symbol(1),
                    function: draft_symbol(6),
                },
            ],
        },
        response: TransactionResponseSpec::default(),
    };
    let prepared = workspace
        .prepare_transaction(&request)
        .expect("cross-module call");
    assert!(query::workspace_blockers(&prepared.snapshot).is_empty());
}

#[test]
fn failed_batches_and_validate_only_do_not_consume_node_ids() {
    let id = WorkspaceId::from_bytes([11; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let first = commit(&mut workspace, &create_package_and_module(id)).expect("first commit");
    let module = first.returned_bindings[1].1;
    assert_eq!(module.serial(), 3);

    let failed = Transaction {
        workspace: id,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: NodeTarget::Existing(module),
                name: "duplicate".to_owned(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: None,
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(4),
                module: NodeTarget::Existing(module),
                name: "duplicate".to_owned(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: None,
            },
        ],
    };
    let error = workspace
        .prepare_transaction(&request(&failed))
        .expect_err("duplicate names must reject");
    assert_eq!(error.code, ErrorCode::DuplicateName);
    assert_eq!(workspace.head_revision(), Revision::new(1));
    assert_eq!(workspace.head().expect("head").next_serial(), 4);

    let validate_only = Transaction {
        workspace: id,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::ValidateOnly,
        operations: vec![TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(5),
            module: NodeTarget::Existing(module),
            name: "function".to_owned(),
            parameters: Vec::new(),
            result: SemanticType::I64.into(),
            body: None,
        }],
    };
    let predicted = commit(&mut workspace, &validate_only).expect("validate only");
    assert_eq!(predicted.returned_bindings[0].1.serial(), 4);
    assert_eq!(workspace.head_revision(), Revision::new(1));

    let mut real = validate_only;
    real.mode = TransactionMode::Commit;
    let committed = commit(&mut workspace, &real).expect("real commit");
    assert_eq!(
        committed.returned_bindings[0].1,
        predicted.returned_bindings[0].1
    );
    assert_eq!(workspace.head_revision(), Revision::new(2));
}

#[test]
fn deletion_tombstones_identity_and_old_snapshots_retain_nodes() {
    let id = WorkspaceId::from_bytes([12; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let first = commit(&mut workspace, &create_package_and_module(id)).expect("first commit");
    let module = first.returned_bindings[1].1;
    let create = Transaction {
        workspace: id,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(3),
            module: NodeTarget::Existing(module),
            name: "function".to_owned(),
            parameters: Vec::new(),
            result: SemanticType::I64.into(),
            body: None,
        }],
    };
    let created = commit(&mut workspace, &create).expect("create function");
    let function = created.returned_bindings[0].1;
    assert_eq!(function.serial(), 4);

    let delete = Transaction {
        workspace: id,
        base_revision: Revision::new(2),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::DeleteOwnedSubtree {
            root: NodeTarget::Existing(function),
        }],
    };
    commit(&mut workspace, &delete).expect("delete function");
    assert!(
        workspace
            .snapshot(Revision::new(2))
            .expect("old snapshot")
            .node(function)
            .is_ok()
    );
    let current = workspace.head().expect("current snapshot");
    assert!(current.node(function).is_err());
    assert!(current.contains_tombstone(function.serial()));

    let replacement = Transaction {
        workspace: id,
        base_revision: Revision::new(3),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(4),
            module: NodeTarget::Existing(module),
            name: "replacement".to_owned(),
            parameters: Vec::new(),
            result: SemanticType::I64.into(),
            body: None,
        }],
    };
    let replacement = commit(&mut workspace, &replacement).expect("replacement function");
    assert_eq!(replacement.returned_bindings[0].1.serial(), 5);
}

#[test]
fn large_user_controlled_subtree_deletion_uses_an_explicit_work_stack() {
    let id = WorkspaceId::from_bytes([15; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let package = DraftSymbol::generated(1);
    let module = DraftSymbol::generated(2);
    let function = DraftSymbol::generated(3);
    let first_value = DraftSymbol::generated(6);
    let body_operations = (0..10_000_u32)
        .map(|offset| ExpressionDraft {
            symbol: Some(DraftSymbol::generated(6 + offset)),
            operation: ExpressionKindDraft::ConstI64(i64::from(offset)),
        })
        .collect();
    let operations = vec![
        TransactionOp::CreatePackage {
            symbol: package,
            name: "package".to_owned(),
        },
        TransactionOp::CreateModule {
            symbol: module,
            package: NodeTarget::Draft(package),
            name: "module".to_owned(),
        },
        TransactionOp::CreateFunction {
            symbol: function,
            module: NodeTarget::Draft(module),
            name: "main".to_owned(),
            parameters: Vec::new(),
            result: SemanticType::I64.into(),
            body: Some(FunctionBodyDraft {
                operations: body_operations,
                return_value: ValueDraft::OperationResult {
                    operation: NodeTarget::Draft(first_value),
                    output: 0,
                },
            }),
        },
        TransactionOp::SetEntryFunction {
            package: NodeTarget::Draft(package),
            function: NodeTarget::Draft(function),
        },
    ];
    let create = Transaction {
        workspace: id,
        base_revision: Revision::INITIAL,
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations,
    };
    let created = commit(&mut workspace, &create).expect("large graph commit");
    let package_id = created.returned_bindings[0].1;
    assert_eq!(workspace.head().expect("head").node_count(), 10_007);

    let delete = Transaction {
        workspace: id,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::DeleteOwnedSubtree {
            root: NodeTarget::Existing(package_id),
        }],
    };
    commit(&mut workspace, &delete).expect("iterative subtree deletion");
    assert_eq!(workspace.head().expect("head").node_count(), 1);
    assert!(
        workspace
            .head()
            .expect("head")
            .contains_tombstone(package_id.serial())
    );
}

fn incomplete_program(id: WorkspaceId) -> Transaction {
    let local = NodeTarget::Draft;
    let value = |symbol| ValueDraft::OperationResult {
        operation: local(symbol),
        output: 0,
    };
    Transaction {
        workspace: id,
        base_revision: Revision::INITIAL,
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![
            TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(1),
                name: "app".to_owned(),
            },
            TransactionOp::CreateModule {
                symbol: DraftSymbol::generated(2),
                package: local(DraftSymbol::generated(1)),
                name: "root".to_owned(),
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: local(DraftSymbol::generated(2)),
                name: "main".to_owned(),
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
                        ExpressionDraft {
                            symbol: Some(DraftSymbol::generated(10)),
                            operation: ExpressionKindDraft::ConstI64(99),
                        },
                    ],
                    return_value: value(DraftSymbol::generated(9)),
                }),
            },
            TransactionOp::SetEntryFunction {
                package: local(DraftSymbol::generated(1)),
                function: local(DraftSymbol::generated(3)),
            },
        ],
    }
}

fn prepared_operation_owner(snapshot: &Snapshot, operation: NodeId) -> NodeId {
    match snapshot.node(operation).expect("operation") {
        Node::Operation { owner, .. } => *owner,
        _ => panic!("operation kind"),
    }
}

fn binding(receipt: &TransactionReceipt, symbol: u32) -> NodeId {
    receipt
        .returned_bindings
        .iter()
        .find_map(|(candidate, node)| (candidate.generated_number() == symbol).then_some(*node))
        .expect("selected binding")
}

#[test]
fn response_projection_is_selected_bounded_and_validate_only_is_predictive() {
    let id = WorkspaceId::from_bytes([0x71; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let transaction = create_package_and_module(id);
    let selected = ApplyTransactionRequest {
        transaction: transaction.clone(),
        response: TransactionResponseSpec {
            return_symbols: vec![DraftSymbol::generated(2)],
        },
    };
    let prepared = workspace
        .prepare_transaction(&selected)
        .expect("selected receipt");
    assert_eq!(prepared.receipt.created_count, 2);
    assert_eq!(prepared.receipt.returned_bindings.len(), 1);
    assert_eq!(
        prepared.receipt.returned_bindings[0].0,
        DraftSymbol::generated(2)
    );

    for return_symbols in [
        vec![DraftSymbol::generated(1), DraftSymbol::generated(1)],
        vec![DraftSymbol::generated(3)],
    ] {
        let invalid = ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: TransactionResponseSpec { return_symbols },
        };
        assert_eq!(
            workspace
                .prepare_transaction(&invalid)
                .expect_err("invalid response projection")
                .code,
            ErrorCode::InvalidDraftSymbol
        );
    }

    let mut too_many = Vec::new();
    for value in 0..=MAX_RETURNED_BINDINGS {
        too_many.push(DraftSymbol::generated(
            u32::try_from(value).expect("symbol"),
        ));
    }
    let invalid = ApplyTransactionRequest {
        transaction: transaction.clone(),
        response: TransactionResponseSpec {
            return_symbols: too_many,
        },
    };
    assert_eq!(
        workspace
            .prepare_transaction(&invalid)
            .expect_err("oversized response projection")
            .code,
        ErrorCode::PolicyExceeded
    );

    let mut validate = selected.clone();
    validate.transaction.mode = TransactionMode::ValidateOnly;
    let predicted = workspace
        .prepare_transaction(&validate)
        .expect("validate-only receipt")
        .receipt;
    assert!(!predicted.published);
    let mut commit_request = validate.clone();
    commit_request.transaction.mode = TransactionMode::Commit;
    let committed = workspace
        .prepare_transaction(&commit_request)
        .expect("commit receipt")
        .receipt;
    let mut expected = predicted;
    expected.published = true;
    assert_eq!(committed, expected);

    validate.transaction.idempotency_key = Some(IdempotencyKey::from_bytes([1; 16]));
    assert_eq!(
        workspace
            .prepare_transaction(&validate)
            .expect_err("validate-only idempotency")
            .code,
        ErrorCode::InvalidOperand
    );
}

#[test]
fn change_digest_includes_exact_scalar_details() {
    let id = WorkspaceId::from_bytes([0x76; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
    let two = binding(&created, 7);
    let edit = |value| Transaction {
        workspace: id,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::ReplaceOperation {
            operation: NodeTarget::Existing(two),
            replacement: OperationDraft::ConstI64(value),
        }],
    };
    let three = workspace
        .prepare_transaction(&request(&edit(3)))
        .expect("replace with three")
        .receipt;
    let four = workspace
        .prepare_transaction(&request(&edit(4)))
        .expect("replace with four")
        .receipt;
    assert_eq!(three.change_count, four.change_count);
    assert_ne!(three.change_digest, four.change_digest);
    assert_ne!(three.hash, four.hash);
}

#[test]
fn same_typed_nominal_definition_changes_are_classified_and_hashed() {
    let id = WorkspaceId::from_bytes([0x78; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let created = commit(
        &mut workspace,
        &Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "p".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: draft_symbol(1),
                    name: "m".into(),
                },
                TransactionOp::CreateProductType {
                    symbol: DraftSymbol::generated(3),
                    module: draft_symbol(2),
                    name: "Pair".into(),
                    fields: vec![
                        ProductFieldDraft {
                            symbol: DraftSymbol::generated(4),
                            name: "left".into(),
                            ty: TypeDraft::I64,
                        },
                        ProductFieldDraft {
                            symbol: DraftSymbol::generated(5),
                            name: "right".into(),
                            ty: TypeDraft::I64,
                        },
                    ],
                },
                TransactionOp::CreateSumType {
                    symbol: DraftSymbol::generated(6),
                    module: draft_symbol(2),
                    name: "Choice".into(),
                    variants: vec![
                        SumVariantDraft {
                            symbol: DraftSymbol::generated(7),
                            name: "First".into(),
                            payload: None,
                        },
                        SumVariantDraft {
                            symbol: DraftSymbol::generated(8),
                            name: "Second".into(),
                            payload: None,
                        },
                    ],
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(9),
                    module: draft_symbol(2),
                    name: "main".into(),
                    parameters: Vec::new(),
                    result: TypeDraft::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            draft_expression(10, ExpressionKindDraft::ConstI64(1)),
                            draft_expression(
                                11,
                                ExpressionKindDraft::ConstructProduct {
                                    product: draft_symbol(3),
                                    fields: vec![
                                        ProductFieldValueDraft {
                                            field: draft_symbol(4),
                                            value: draft_result(10),
                                        },
                                        ProductFieldValueDraft {
                                            field: draft_symbol(5),
                                            value: draft_result(10),
                                        },
                                    ],
                                },
                            ),
                            draft_expression(
                                12,
                                ExpressionKindDraft::ProjectField {
                                    value: draft_result(11),
                                    field: draft_symbol(4),
                                },
                            ),
                            draft_expression(
                                13,
                                ExpressionKindDraft::ConstructVariant {
                                    variant: draft_symbol(7),
                                    payload: None,
                                },
                            ),
                        ],
                        return_value: draft_result(12),
                    }),
                },
            ],
        },
    )
    .expect("nominal definitions");
    let field_before = binding(&created, 4);
    let field_after = binding(&created, 5);
    let variant_before = binding(&created, 7);
    let variant_after = binding(&created, 8);
    let product_value = binding(&created, 11);
    let projection = binding(&created, 12);
    let construction = binding(&created, 13);
    let base = workspace.snapshot(Revision::new(1)).expect("base");

    let cases = [
        (
            projection,
            OperationDraft::ProjectField {
                value: ValueDraft::OperationResult {
                    operation: NodeTarget::Existing(product_value),
                    output: 0,
                },
                field: NodeTarget::Existing(field_after),
            },
            field_before,
            field_after,
        ),
        (
            construction,
            OperationDraft::ConstructVariant {
                variant: NodeTarget::Existing(variant_after),
                payload: None,
            },
            variant_before,
            variant_after,
        ),
    ];
    for (operation, replacement, before, after) in cases {
        let prepared = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction: Transaction {
                    workspace: id,
                    base_revision: Revision::new(1),
                    idempotency_key: None,
                    mode: TransactionMode::ValidateOnly,
                    operations: vec![TransactionOp::ReplaceOperation {
                        operation: NodeTarget::Existing(operation),
                        replacement,
                    }],
                },
                response: TransactionResponseSpec::default(),
            })
            .expect("same-typed definition replacement");
        let semantic_diff = diff::between(base, &prepared.snapshot);
        assert_eq!(semantic_diff, diff::between(base, &prepared.snapshot));
        assert_ne!(semantic_diff.digest.as_bytes(), [0; 32]);
        assert_eq!(prepared.receipt.change_digest, semantic_diff.digest);
        assert!(semantic_diff.changes.iter().any(|change| {
            change.node == operation
                && matches!(
                    change.kind,
                    crate::diff::ChangeKind::DefinitionChanged {
                        before: actual_before,
                        after: actual_after,
                    } if actual_before == before && actual_after == after
                )
        }));
    }
}

#[test]
fn change_digest_distinguishes_refinement_payloads_and_same_typed_operands() {
    let id = WorkspaceId::from_bytes([0x77; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
    let forty = binding(&created, 6);
    let two = binding(&created, 7);
    let hole = binding(&created, 9);
    let refinement = |value| Transaction {
        workspace: id,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::RefineHole {
            hole: NodeTarget::Existing(hole),
            replacement: OperationDraft::ConstI64(value),
        }],
    };
    let two_refinement = workspace
        .prepare_transaction(&request(&refinement(2)))
        .expect("refine to two");
    let three_refinement = workspace
        .prepare_transaction(&request(&refinement(3)))
        .expect("refine to three");
    assert_ne!(two_refinement.receipt.hash, three_refinement.receipt.hash);
    assert_ne!(
        two_refinement.receipt.change_digest,
        three_refinement.receipt.change_digest
    );
    let two_change = diff::between(
        workspace.snapshot(Revision::new(1)).expect("base"),
        &two_refinement.snapshot,
    );
    assert!(two_change.changes.iter().any(|change| {
        matches!(
            &change.kind,
            crate::diff::ChangeKind::OperationRefined {
                replacement: OperationKind::ConstI64(2),
                ..
            }
        )
    }));

    let add_refinement = Transaction {
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
    commit(&mut workspace, &add_refinement).expect("publish add refinement");
    let replacement = |index, operation| Transaction {
        workspace: id,
        base_revision: Revision::new(2),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::ReplaceOperand {
            operation: NodeTarget::Existing(hole),
            index,
            value: ValueDraft::OperationResult {
                operation: NodeTarget::Existing(operation),
                output: 0,
            },
        }],
    };
    let replace_left = workspace
        .prepare_transaction(&request(&replacement(0, two)))
        .expect("replace left operand");
    let replace_right = workspace
        .prepare_transaction(&request(&replacement(1, forty)))
        .expect("replace right operand");
    assert_ne!(replace_left.receipt.hash, replace_right.receipt.hash);
    assert_ne!(
        replace_left.receipt.change_digest,
        replace_right.receipt.change_digest
    );
    let left_diff = diff::between(
        workspace.snapshot(Revision::new(2)).expect("refined base"),
        &replace_left.snapshot,
    );
    assert!(left_diff.changes.iter().any(|change| {
        matches!(
            change.kind,
            crate::diff::ChangeKind::OperandChanged {
                index: 0,
                before: Some(ValueRef::OperationResult { operation, .. }),
                after: Some(ValueRef::OperationResult {
                    operation: replacement,
                    ..
                }),
            } if operation == forty && replacement == two
        )
    }));
}

#[test]
fn create_then_delete_returns_selected_tombstoned_identity_and_explicit_change() {
    let id = WorkspaceId::from_bytes([0x74; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let transaction = Transaction {
        workspace: id,
        base_revision: Revision::INITIAL,
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![
            TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(1),
                name: "temporary".to_owned(),
            },
            TransactionOp::DeleteOwnedSubtree {
                root: NodeTarget::Draft(DraftSymbol::generated(1)),
            },
        ],
    };
    let prepared = workspace
        .prepare_transaction(&request(&transaction))
        .expect("create then delete");
    let allocated = binding(&prepared.receipt, 1);
    assert!(prepared.snapshot.contains_tombstone(allocated.serial()));
    assert!(prepared.receipt.change_count > 0);
    let before = workspace.head().expect("before");
    let semantic_diff = diff::between(before, &prepared.snapshot);
    assert!(semantic_diff.changes.iter().any(|change| {
        change.node == allocated
            && matches!(change.kind, crate::diff::ChangeKind::AllocatedAndTombstoned)
    }));
}

#[test]
fn hole_refinement_preserves_identity_position_use_history_and_diff() {
    let id = WorkspaceId::from_bytes([0x72; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
    let hole = binding(&created, 9);
    let forty = binding(&created, 6);
    let two = binding(&created, 7);
    let block = prepared_operation_owner(workspace.head().expect("head"), hole);
    let return_operation = match workspace.head().expect("head").node(block).expect("block") {
        Node::Block {
            terminator: Some(terminator),
            ..
        } => *terminator,
        _ => panic!("block terminator"),
    };
    let old = workspace
        .snapshot(Revision::new(1))
        .expect("old snapshot")
        .clone();
    let refine = Transaction {
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
    let refined = commit(&mut workspace, &refine).expect("refine hole");
    assert_eq!(refined.created_count, 0);
    assert!(!refined.complete_before);
    assert!(refined.complete_after);
    let current = workspace.head().expect("refined snapshot");
    assert!(matches!(
        old.node(hole).expect("old hole"),
        Node::Operation {
            operation: OperationKind::Hole { .. },
            ..
        }
    ));
    assert!(matches!(
        current.node(hole).expect("refined operation"),
        Node::Operation {
            operation: OperationKind::AddI64 { .. },
            ..
        }
    ));
    let Node::Block { operations, .. } = current.node(block).expect("block") else {
        panic!("block kind");
    };
    assert_eq!(operations.iter().position(|id| *id == hole), Some(3));
    let Node::Operation {
        operation: OperationKind::Return { value },
        ..
    } = current.node(return_operation).expect("return")
    else {
        panic!("return kind");
    };
    assert_eq!(
        *value,
        ValueRef::OperationResult {
            operation: hole,
            output: 0,
        }
    );
    let semantic_diff = diff::between(&old, current);
    assert_eq!(semantic_diff.change_count(), refined.change_count);
    assert_eq!(semantic_diff.digest, refined.change_digest);
    assert!(semantic_diff.changes.iter().any(|change| {
        change.node == hole
            && matches!(
                change.kind,
                crate::diff::ChangeKind::OperationRefined {
                    before: crate::schema::OperationCode::Hole,
                    after: crate::schema::OperationCode::AddI64,
                    result_type: SemanticType::I64,
                    ..
                }
            )
    }));
}

#[test]
fn hole_refinement_to_identity_targeted_call_uses_snapshot_signature() {
    let id = WorkspaceId::from_bytes([0x79; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
    let module = binding(&created, 2);
    let hole = binding(&created, 9);
    let transaction = Transaction {
        workspace: id,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(12),
                module: NodeTarget::Existing(module),
                name: "callee".into(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: None,
            },
            TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::Call {
                    function: NodeTarget::Draft(DraftSymbol::generated(12)),
                    arguments: vec![],
                },
            },
        ],
    };
    let prepared = workspace
        .prepare_transaction(&request(&transaction))
        .expect("call refinement");
    let Node::Operation {
        operation: OperationKind::Call {
            function,
            arguments,
        },
        ..
    } = prepared.snapshot.node(hole).expect("refined call")
    else {
        panic!("call refinement kind")
    };
    assert_eq!(*function, binding(&prepared.receipt, 12));
    assert!(arguments.is_empty());
    assert!(
        diff::between(workspace.head().expect("old head"), &prepared.snapshot)
            .changes
            .iter()
            .any(|change| matches!(
                change.kind,
                crate::diff::ChangeKind::OperationRefined {
                    after: crate::schema::OperationCode::Call,
                    ..
                }
            ))
    );
}

#[test]
fn structured_expansion_is_depth_first_predictive_and_supports_forward_calls() {
    let id = WorkspaceId::from_bytes([0x7a; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let local = |symbol| NodeTarget::Draft(DraftSymbol::generated(symbol));
    let result = |symbol| ValueDraft::OperationResult {
        operation: local(symbol),
        output: 0,
    };
    let block_argument = |symbol| ValueDraft::BlockArgument(local(symbol));
    let transaction = Transaction {
        workspace: id,
        base_revision: Revision::INITIAL,
        idempotency_key: None,
        mode: TransactionMode::ValidateOnly,
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
                symbol: DraftSymbol::generated(5),
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
                            operation: ExpressionKindDraft::LtI64 {
                                lhs: result(6),
                                rhs: result(7),
                            },
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
                                        operation: ExpressionKindDraft::AddI64 {
                                            lhs: block_argument(11),
                                            rhs: block_argument(10),
                                        },
                                    }],
                                    yield_value: result(12),
                                },
                            },
                        },
                        ExpressionDraft {
                            symbol: Some(DraftSymbol::generated(13)),
                            operation: ExpressionKindDraft::If {
                                condition: result(8),
                                result: SemanticType::I64.into(),
                                then_body: YieldingBodyDraft {
                                    operations: vec![ExpressionDraft {
                                        symbol: Some(DraftSymbol::generated(14)),
                                        operation: ExpressionKindDraft::Call {
                                            function: local(20),
                                            arguments: vec![result(9)],
                                        },
                                    }],
                                    yield_value: result(14),
                                },
                                else_body: YieldingBodyDraft {
                                    operations: vec![ExpressionDraft {
                                        symbol: Some(DraftSymbol::generated(15)),
                                        operation: ExpressionKindDraft::ConstI64(0),
                                    }],
                                    yield_value: result(15),
                                },
                            },
                        },
                    ],
                    return_value: result(13),
                }),
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(20),
                module: local(2),
                name: "later".into(),
                parameters: vec![FunctionParameterDraft {
                    symbol: DraftSymbol::generated(21),
                    name: "value".into(),
                    ty: SemanticType::I64.into(),
                }],
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: Vec::new(),
                    return_value: ValueDraft::FunctionParameter(local(21)),
                }),
            },
            TransactionOp::SetEntryFunction {
                package: local(1),
                function: local(5),
            },
        ],
    };
    let response = TransactionResponseSpec {
        return_symbols: [1, 2, 5, 6, 9, 10, 11, 12, 13, 14, 15, 20, 21]
            .into_iter()
            .map(DraftSymbol::generated)
            .collect(),
    };
    let predicted = workspace
        .prepare_transaction(&ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: response.clone(),
        })
        .expect("validate structured");
    assert_eq!(predicted.receipt.created_count, 5);
    assert_eq!(binding(&predicted.receipt, 5).serial(), 4);
    assert_eq!(
        binding(&predicted.receipt, 10).local_function_serial(),
        Some(4)
    );
    assert_eq!(binding(&predicted.receipt, 10).local_ordinal(), Some(9));
    assert_eq!(
        binding(&predicted.receipt, 11).local_function_serial(),
        Some(4)
    );
    assert_eq!(binding(&predicted.receipt, 11).local_ordinal(), Some(10));
    assert_eq!(binding(&predicted.receipt, 20).serial(), 5);
    let Node::Operation {
        operation: OperationKind::Call { function, .. },
        ..
    } = predicted
        .snapshot
        .node(binding(&predicted.receipt, 14))
        .expect("call")
    else {
        panic!("call kind")
    };
    assert_eq!(*function, binding(&predicted.receipt, 20));
    let mut committed_transaction = transaction;
    committed_transaction.mode = TransactionMode::Commit;
    let committed = workspace
        .prepare_transaction(&ApplyTransactionRequest {
            transaction: committed_transaction,
            response,
        })
        .expect("commit structured");
    let mut expected = predicted.receipt;
    expected.published = true;
    assert_eq!(committed.receipt, expected);
}

#[test]
fn structured_symbols_reject_zero_duplicates_undeclared_and_private_selection_atomically() {
    let id = WorkspaceId::from_bytes([0x7b; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let base_transaction = |expression: ExpressionDraft| Transaction {
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
                package: NodeTarget::Draft(DraftSymbol::generated(1)),
                name: "root".into(),
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: NodeTarget::Draft(DraftSymbol::generated(2)),
                name: "main".into(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: vec![expression],
                    return_value: ValueDraft::OperationResult {
                        operation: NodeTarget::Draft(DraftSymbol::generated(4)),
                        output: 0,
                    },
                }),
            },
        ],
    };
    let unchecked = |transaction| ApplyTransactionRequest {
        transaction,
        response: TransactionResponseSpec::default(),
    };
    let invalid = serde_json::from_str::<DraftSymbol>("\"\"").expect("raw invalid symbol");
    let zero = base_transaction(ExpressionDraft {
        symbol: Some(invalid),
        operation: ExpressionKindDraft::ConstI64(1),
    });
    let error = workspace
        .prepare_transaction(&unchecked(zero))
        .expect_err("zero");
    assert_eq!(error.code, ErrorCode::InvalidDraftSymbol);
    assert_eq!(error.operation_index, Some(2));
    assert_eq!(error.draft_symbol, Some(invalid));
    for raw in ["Bad", &"x".repeat(crate::ids::MAX_DRAFT_SYMBOL_BYTES + 1)] {
        let encoded = serde_json::to_string(raw).expect("invalid symbol JSON");
        let invalid = serde_json::from_str::<DraftSymbol>(&encoded)
            .expect("raw invalid symbol remains typed for semantic diagnostics");
        let error = workspace
            .prepare_transaction(&unchecked(base_transaction(ExpressionDraft {
                symbol: Some(invalid),
                operation: ExpressionKindDraft::ConstI64(1),
            })))
            .expect_err("invalid symbol");
        assert_eq!(error.code, ErrorCode::InvalidDraftSymbol);
        assert_eq!(error.operation_index, Some(2));
        assert_eq!(error.draft_symbol, Some(invalid));
    }
    let duplicate = base_transaction(ExpressionDraft {
        symbol: Some(DraftSymbol::generated(3)),
        operation: ExpressionKindDraft::ConstI64(1),
    });
    let error = workspace
        .prepare_transaction(&unchecked(duplicate))
        .expect_err("duplicate");
    assert_eq!(error.code, ErrorCode::DuplicateDraftSymbol);
    assert_eq!(error.operation_index, Some(2));
    assert_eq!(error.draft_symbol, Some(DraftSymbol::generated(3)));
    let undeclared = base_transaction(ExpressionDraft {
        symbol: Some(DraftSymbol::generated(4)),
        operation: ExpressionKindDraft::AddI64 {
            lhs: ValueDraft::OperationResult {
                operation: NodeTarget::Draft(DraftSymbol::generated(99)),
                output: 0,
            },
            rhs: ValueDraft::OperationResult {
                operation: NodeTarget::Draft(DraftSymbol::generated(4)),
                output: 0,
            },
        },
    });
    let error = workspace
        .prepare_transaction(&unchecked(undeclared))
        .expect_err("undeclared");
    assert_eq!(error.code, ErrorCode::InvalidDraftSymbol);
    assert_eq!(error.operation_index, Some(2));
    assert_eq!(error.draft_symbol, Some(DraftSymbol::generated(99)));
    let valid = base_transaction(ExpressionDraft {
        symbol: Some(DraftSymbol::generated(4)),
        operation: ExpressionKindDraft::ConstI64(1),
    });
    let private = ApplyTransactionRequest {
        transaction: valid,
        response: TransactionResponseSpec {
            return_symbols: vec![DraftSymbol::generated(u32::MAX)],
        },
    };
    assert_eq!(
        workspace
            .prepare_transaction(&private)
            .expect_err("private binding")
            .code,
        ErrorCode::InvalidDraftSymbol
    );
    assert_eq!(workspace.head().expect("head").next_serial(), 2);
}

#[test]
fn canonical_allocation_errors_remap_to_public_source_and_explicit_symbol() {
    let id = WorkspaceId::from_bytes([0x7a; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let symbol = DraftSymbol::generated(77);
    let edits = vec![
        CanonicalEdit::CreatePackage {
            symbol,
            name: "first".into(),
        },
        CanonicalEdit::CreatePackage {
            symbol,
            name: "duplicate".into(),
        },
    ];
    let error = allocate_symbols(
        workspace.head().expect("head"),
        &edits,
        &[3, 8],
        &BTreeSet::from([symbol]),
        &BTreeMap::new(),
    )
    .expect_err("duplicate canonical allocation");
    assert_eq!(error.code, ErrorCode::DuplicateDraftSymbol);
    assert_eq!(error.operation_index, Some(8));
    assert_eq!(error.draft_symbol, Some(symbol));
}

#[test]
fn insert_expression_rejects_staged_block_and_anchor_with_public_source_atomically() {
    let id = WorkspaceId::from_bytes([0x7e; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let local = |symbol| NodeTarget::Draft(DraftSymbol::generated(symbol));
    let staged_block = NodeId::new(id, 6).expect("predicted staged block");
    let transaction = Transaction {
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
                    operations: vec![ExpressionDraft {
                        symbol: Some(DraftSymbol::generated(4)),
                        operation: ExpressionKindDraft::ConstI64(1),
                    }],
                    return_value: ValueDraft::OperationResult {
                        operation: local(4),
                        output: 0,
                    },
                }),
            },
            TransactionOp::InsertExpression {
                block: staged_block,
                before: None,
                expression: ExpressionDraft {
                    symbol: Some(DraftSymbol::generated(5)),
                    operation: ExpressionKindDraft::ConstI64(2),
                },
            },
        ],
    };
    let error = workspace
        .prepare_transaction(&ApplyTransactionRequest {
            transaction,
            response: TransactionResponseSpec::default(),
        })
        .expect_err("staged block");
    assert_eq!(error.operation_index, Some(3));
    assert_eq!(error.target, Some(staged_block));
    assert_eq!(workspace.head().expect("head").next_serial(), 2);

    let committed_id = WorkspaceId::from_bytes([0x7f; 16]);
    let mut committed = Workspace::new(committed_id).expect("workspace");
    let created = commit(&mut committed, &incomplete_program(committed_id)).expect("fixture");
    let hole = binding(&created, 9);
    let block = prepared_operation_owner(committed.head().expect("head"), hole);
    let predicted_anchor = NodeId::new(
        committed.id(),
        committed.head().expect("head").next_serial(),
    )
    .expect("predicted anchor");
    let transaction = Transaction {
        workspace: committed.id(),
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![
            TransactionOp::InsertExpression {
                block,
                before: Some(hole),
                expression: ExpressionDraft {
                    symbol: Some(DraftSymbol::generated(100)),
                    operation: ExpressionKindDraft::ConstI64(1),
                },
            },
            TransactionOp::InsertExpression {
                block,
                before: Some(predicted_anchor),
                expression: ExpressionDraft {
                    symbol: Some(DraftSymbol::generated(101)),
                    operation: ExpressionKindDraft::ConstI64(2),
                },
            },
        ],
    };
    let frontier = committed.head().expect("head").next_serial();
    let error = committed
        .prepare_transaction(&ApplyTransactionRequest {
            transaction,
            response: TransactionResponseSpec::default(),
        })
        .expect_err("staged anchor");
    assert_eq!(error.operation_index, Some(1));
    assert_eq!(error.target, Some(predicted_anchor));
    assert_eq!(committed.head().expect("head").next_serial(), frontier);
}

#[test]
fn preallocation_scan_maps_wrong_local_call_target_to_public_operation() {
    let id = WorkspaceId::from_bytes([0x80; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let local = |symbol| NodeTarget::Draft(DraftSymbol::generated(symbol));
    let transaction = Transaction {
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
                name: "bad".into(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: vec![ExpressionDraft {
                        symbol: Some(DraftSymbol::generated(4)),
                        operation: ExpressionKindDraft::Call {
                            function: local(1),
                            arguments: Vec::new(),
                        },
                    }],
                    return_value: ValueDraft::OperationResult {
                        operation: local(4),
                        output: 0,
                    },
                }),
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(5),
                module: local(2),
                name: "later".into(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: None,
            },
        ],
    };
    let error = workspace
        .prepare_transaction(&ApplyTransactionRequest {
            transaction,
            response: TransactionResponseSpec::default(),
        })
        .expect_err("bad call target");
    assert_eq!(error.code, ErrorCode::WrongKind);
    assert_eq!(error.operation_index, Some(2));
    assert_eq!(error.draft_symbol, Some(DraftSymbol::generated(1)));
    assert_eq!(workspace.head().expect("head").next_serial(), 2);
}

#[test]
fn structured_request_depth_item_and_output_policies_reject_before_allocation() {
    let id = WorkspaceId::from_bytes([0x7d; 16]);
    let block = NodeId::new(id, 2).expect("block ID");
    let existing = NodeTarget::Existing(block);

    let top_level = (0..=MAX_STRUCTURED_DRAFT_ITEMS)
        .map(|_| TransactionOp::RenameNode {
            node: existing,
            name: "x".into(),
        })
        .collect::<Vec<_>>();
    let error = scan_explicit_symbols(&top_level).expect_err("top-level item policy");
    assert_eq!(error.code, ErrorCode::PolicyExceeded);
    assert_eq!(
        error.operation_index,
        Some(MAX_STRUCTURED_DRAFT_ITEMS as u32)
    );

    let mut mixed = (0..MAX_STRUCTURED_DRAFT_ITEMS - 2)
        .map(|_| TransactionOp::RenameNode {
            node: existing,
            name: "x".into(),
        })
        .collect::<Vec<_>>();
    mixed.push(TransactionOp::InsertExpression {
        block,
        before: None,
        expression: ExpressionDraft {
            symbol: Some(DraftSymbol::generated(40_000)),
            operation: ExpressionKindDraft::Call {
                function: existing,
                arguments: vec![ValueDraft::FunctionParameter(existing); 3],
            },
        },
    });
    let error = scan_explicit_symbols(&mixed).expect_err("mixed item policy");
    assert_eq!(error.code, ErrorCode::PolicyExceeded);
    assert_eq!(
        error.operation_index,
        Some((MAX_STRUCTURED_DRAFT_ITEMS - 2) as u32)
    );

    let mut expression = ExpressionDraft {
        symbol: Some(DraftSymbol::generated(1)),
        operation: ExpressionKindDraft::ConstI64(1),
    };
    for depth in 0..=MAX_STRUCTURED_DRAFT_DEPTH {
        let inner_symbol = expression.symbol;
        let else_symbol = DraftSymbol::generated(10_000 + depth as u32);
        expression = ExpressionDraft {
            symbol: Some(DraftSymbol::generated(20_000 + depth as u32)),
            operation: ExpressionKindDraft::If {
                condition: ValueDraft::OperationResult {
                    operation: existing,
                    output: 0,
                },
                result: SemanticType::I64.into(),
                then_body: YieldingBodyDraft {
                    operations: vec![expression],
                    yield_value: ValueDraft::OperationResult {
                        operation: NodeTarget::Draft(inner_symbol.expect("bound expression")),
                        output: 0,
                    },
                },
                else_body: YieldingBodyDraft {
                    operations: vec![ExpressionDraft {
                        symbol: Some(else_symbol),
                        operation: ExpressionKindDraft::ConstI64(0),
                    }],
                    yield_value: ValueDraft::OperationResult {
                        operation: NodeTarget::Draft(else_symbol),
                        output: 0,
                    },
                },
            },
        };
    }
    let too_deep = [TransactionOp::InsertExpression {
        block,
        before: None,
        expression,
    }];
    assert_eq!(
        scan_explicit_symbols(&too_deep)
            .expect_err("depth policy")
            .code,
        ErrorCode::PolicyExceeded
    );

    let oversized = [TransactionOp::InsertExpression {
        block,
        before: None,
        expression: ExpressionDraft {
            symbol: Some(DraftSymbol::generated(1)),
            operation: ExpressionKindDraft::Call {
                function: existing,
                arguments: vec![
                    ValueDraft::FunctionParameter(existing);
                    MAX_STRUCTURED_DRAFT_ITEMS + 1
                ],
            },
        },
    }];
    assert_eq!(
        scan_explicit_symbols(&oversized)
            .expect_err("item policy")
            .code,
        ErrorCode::PolicyExceeded
    );

    for fine_grained in [
        TransactionOp::ReplaceOperation {
            operation: existing,
            replacement: OperationDraft::Call {
                function: existing,
                arguments: vec![
                    ValueDraft::FunctionParameter(existing);
                    MAX_STRUCTURED_DRAFT_ITEMS
                ],
            },
        },
        TransactionOp::RefineHole {
            hole: existing,
            replacement: OperationDraft::Call {
                function: existing,
                arguments: vec![
                    ValueDraft::FunctionParameter(existing);
                    MAX_STRUCTURED_DRAFT_ITEMS
                ],
            },
        },
    ] {
        let request = ApplyTransactionRequest {
            transaction: Transaction {
                workspace: id,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![fine_grained],
            },
            response: TransactionResponseSpec::default(),
        };
        let workspace = Workspace::new(id).expect("workspace");
        let before = artifact::encode(workspace.head().expect("head")).expect("artifact");
        let error = workspace
            .prepare_transaction(&request)
            .expect_err("fine-grained call aggregate policy");
        assert_eq!(error.code, ErrorCode::PolicyExceeded);
        assert_eq!(workspace.head_revision(), Revision::INITIAL);
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
        assert_eq!(
            artifact::encode(workspace.head().expect("head")).expect("artifact"),
            before
        );
    }

    let invalid_output = [TransactionOp::InsertExpression {
        block,
        before: None,
        expression: ExpressionDraft {
            symbol: Some(DraftSymbol::generated(1)),
            operation: ExpressionKindDraft::AddI64 {
                lhs: ValueDraft::OperationResult {
                    operation: existing,
                    output: 1,
                },
                rhs: ValueDraft::OperationResult {
                    operation: existing,
                    output: 0,
                },
            },
        },
    }];
    assert_eq!(
        scan_explicit_symbols(&invalid_output)
            .expect_err("output index")
            .code,
        ErrorCode::InvalidOperand
    );
}

#[test]
fn mutual_function_bodies_resolve_local_calls_in_one_transaction() {
    let id = WorkspaceId::from_bytes([0x7c; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let local = |symbol| NodeTarget::Draft(DraftSymbol::generated(symbol));
    let call_body = |symbol, target| FunctionBodyDraft {
        operations: vec![ExpressionDraft {
            symbol: Some(DraftSymbol::generated(symbol)),
            operation: ExpressionKindDraft::Call {
                function: local(target),
                arguments: Vec::new(),
            },
        }],
        return_value: ValueDraft::OperationResult {
            operation: local(symbol),
            output: 0,
        },
    };
    let transaction = Transaction {
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
                name: "a".into(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(call_body(5, 4)),
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(4),
                module: local(2),
                name: "b".into(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(call_body(6, 3)),
            },
        ],
    };
    let receipt = commit(&mut workspace, &transaction).expect("one-transaction mutual calls");
    let function_a = binding(&receipt, 3);
    let function_b = binding(&receipt, 4);
    assert_eq!(function_a.serial(), 4);
    assert_eq!(function_b.serial(), 5);
    for (call_symbol, owner_function, expected_target) in
        [(5, function_a, function_b), (6, function_b, function_a)]
    {
        let call = binding(&receipt, call_symbol);
        assert_eq!(call.local_function_serial(), Some(owner_function.serial()));
        let Node::Operation {
            operation:
                OperationKind::Call {
                    function,
                    arguments,
                },
            ..
        } = workspace.head().expect("head").node(call).expect("call")
        else {
            panic!("call operation")
        };
        assert_eq!(*function, expected_target);
        assert!(arguments.is_empty());
    }
}

#[test]
fn hole_refinement_can_use_supporting_values_created_before_it_atomically() {
    let id = WorkspaceId::from_bytes([0x75; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
    let forty = binding(&created, 6);
    let hole = binding(&created, 9);
    let block = prepared_operation_owner(workspace.head().expect("head"), hole);
    let support = DraftSymbol::generated(100);
    let transaction = Transaction {
        workspace: id,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![
            TransactionOp::InsertExpression {
                block,
                before: Some(hole),
                expression: ExpressionDraft {
                    symbol: Some(support),
                    operation: ExpressionKindDraft::ConstI64(2),
                },
            },
            TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::AddI64 {
                    lhs: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(forty),
                        output: 0,
                    },
                    rhs: ValueDraft::OperationResult {
                        operation: NodeTarget::Draft(support),
                        output: 0,
                    },
                },
            },
        ],
    };
    let prepared = workspace
        .prepare_transaction(&ApplyTransactionRequest {
            transaction,
            response: TransactionResponseSpec {
                return_symbols: vec![support],
            },
        })
        .expect("atomic support and refinement");
    assert_eq!(prepared.receipt.created_count, 0);
    assert!(prepared.receipt.complete_after);
    let support_id = binding(&prepared.receipt, 100);
    let Node::Block { operations, .. } = prepared.snapshot.node(block).expect("block") else {
        panic!("block kind");
    };
    let support_position = operations
        .iter()
        .position(|id| *id == support_id)
        .expect("support position");
    let hole_position = operations
        .iter()
        .position(|id| *id == hole)
        .expect("hole position");
    assert!(support_position < hole_position);
}

#[test]
fn hole_refinement_rejects_wrong_targets_contracts_types_and_order() {
    let id = WorkspaceId::from_bytes([0x73; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
    let package = binding(&created, 1);
    let forty = binding(&created, 6);
    let boolean = binding(&created, 8);
    let hole = binding(&created, 9);
    let later = binding(&created, 10);
    let value = |operation| ValueDraft::OperationResult {
        operation: NodeTarget::Existing(operation),
        output: 0,
    };
    let cases = [
        (package, OperationDraft::ConstI64(1), ErrorCode::WrongKind),
        (
            forty,
            OperationDraft::ConstI64(1),
            ErrorCode::InvalidOperand,
        ),
        (
            hole,
            OperationDraft::Hole {
                expected: SemanticType::I64.into(),
            },
            ErrorCode::InvalidOperand,
        ),
        (
            hole,
            OperationDraft::Return {
                value: value(forty),
            },
            ErrorCode::InvalidOperand,
        ),
        (
            hole,
            OperationDraft::ConstBool(false),
            ErrorCode::TypeMismatch,
        ),
        (
            hole,
            OperationDraft::AddI64 {
                lhs: value(forty),
                rhs: value(boolean),
            },
            ErrorCode::TypeMismatch,
        ),
        (
            hole,
            OperationDraft::AddI64 {
                lhs: value(forty),
                rhs: value(later),
            },
            ErrorCode::InvalidOperand,
        ),
        (
            hole,
            OperationDraft::AddI64 {
                lhs: value(forty),
                rhs: value(hole),
            },
            ErrorCode::InvalidOperand,
        ),
    ];
    for (target, replacement, expected) in cases {
        let refine = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(target),
                replacement,
            }],
        };
        assert_eq!(
            workspace
                .prepare_transaction(&request(&refine))
                .expect_err("invalid refinement")
                .code,
            expected
        );
        assert_eq!(workspace.head_revision(), Revision::new(1));
        assert!(matches!(
            workspace.head().expect("head").node(hole).expect("hole"),
            Node::Operation {
                operation: OperationKind::Hole { .. },
                ..
            }
        ));
    }
}

#[test]
fn nominal_declarations_resolve_forward_types_and_derive_exact_layouts() {
    let id = WorkspaceId::from_bytes([0x91; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let request = ApplyTransactionRequest {
        transaction: Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::ValidateOnly,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "p".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: draft_symbol(1),
                    name: "m".into(),
                },
                TransactionOp::CreateProductType {
                    symbol: DraftSymbol::generated(3),
                    module: draft_symbol(2),
                    name: "Reading".into(),
                    fields: vec![
                        ProductFieldDraft {
                            symbol: DraftSymbol::generated(4),
                            name: "valid".into(),
                            ty: TypeDraft::Bool,
                        },
                        ProductFieldDraft {
                            symbol: DraftSymbol::generated(5),
                            name: "value".into(),
                            ty: TypeDraft::I64,
                        },
                    ],
                },
                TransactionOp::CreateSumType {
                    symbol: DraftSymbol::generated(6),
                    module: draft_symbol(2),
                    name: "Input".into(),
                    variants: vec![
                        SumVariantDraft {
                            symbol: DraftSymbol::generated(7),
                            name: "missing".into(),
                            payload: None,
                        },
                        SumVariantDraft {
                            symbol: DraftSymbol::generated(8),
                            name: "sample".into(),
                            payload: Some(TypeDraft::Nominal(draft_symbol(3))),
                        },
                    ],
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(9),
                    module: draft_symbol(2),
                    name: "pending".into(),
                    parameters: vec![FunctionParameterDraft {
                        symbol: DraftSymbol::generated(10),
                        name: "input".into(),
                        ty: TypeDraft::Nominal(draft_symbol(6)),
                    }],
                    result: TypeDraft::Nominal(draft_symbol(3)),
                    body: Some(FunctionBodyDraft {
                        operations: vec![draft_expression(
                            11,
                            ExpressionKindDraft::Hole {
                                expected: TypeDraft::Nominal(draft_symbol(3)),
                            },
                        )],
                        return_value: draft_result(11),
                    }),
                },
            ],
        },
        response: TransactionResponseSpec {
            return_symbols: vec![
                DraftSymbol::generated(3),
                DraftSymbol::generated(4),
                DraftSymbol::generated(5),
                DraftSymbol::generated(6),
                DraftSymbol::generated(7),
                DraftSymbol::generated(8),
                DraftSymbol::generated(9),
                DraftSymbol::generated(11),
            ],
        },
    };
    let prepared = workspace
        .prepare_transaction(&request)
        .expect("nominal validate-only");
    assert!(!prepared.receipt.published);
    assert_eq!(workspace.head().expect("head").next_serial(), 2);
    let reading = prepared.receipt.returned_bindings[0].1;
    let input = prepared.receipt.returned_bindings[3].1;
    let Node::Module {
        types, functions, ..
    } = prepared
        .snapshot
        .node(NodeId::new(id, 3).expect("module"))
        .expect("module")
    else {
        panic!("module kind")
    };
    assert_eq!(types, &[reading, input]);
    assert_eq!(functions.len(), 1);
    let layouts = crate::type_layout::derive_layouts(&prepared.snapshot).expect("layouts");
    let crate::type_layout::DerivedLayout::Representable(reading_layout) =
        layouts.get(&reading).expect("reading layout")
    else {
        panic!("representable")
    };
    assert_eq!(
        (
            reading_layout.size,
            reading_layout.align,
            reading_layout.cells
        ),
        (16, 8, 2)
    );
    let crate::type_layout::LayoutShape::Product { fields } = &reading_layout.shape else {
        panic!("product layout")
    };
    assert_eq!(
        fields.iter().map(|field| field.offset).collect::<Vec<_>>(),
        [0, 8]
    );
    let crate::type_layout::DerivedLayout::Representable(input_layout) =
        layouts.get(&input).expect("input layout")
    else {
        panic!("representable")
    };
    assert_eq!(input_layout.cells, 3);
}

#[test]
fn by_value_cycles_and_duplicate_member_names_reject_without_identity_consumption() {
    let id = WorkspaceId::from_bytes([0x92; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let cyclic = structured_semantic_request(
        id,
        vec![
            TransactionOp::CreateProductType {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "A".into(),
                fields: vec![ProductFieldDraft {
                    symbol: DraftSymbol::generated(4),
                    name: "b".into(),
                    ty: TypeDraft::Nominal(draft_symbol(5)),
                }],
            },
            TransactionOp::CreateProductType {
                symbol: DraftSymbol::generated(5),
                module: draft_symbol(2),
                name: "B".into(),
                fields: vec![ProductFieldDraft {
                    symbol: DraftSymbol::generated(6),
                    name: "a".into(),
                    ty: TypeDraft::Nominal(draft_symbol(3)),
                }],
            },
        ],
    );
    let error = workspace.prepare_transaction(&cyclic).expect_err("cycle");
    assert_eq!(error.code, ErrorCode::ByValueTypeCycle);
    assert_eq!(workspace.head().expect("head").next_serial(), 2);

    let duplicate = structured_semantic_request(
        id,
        vec![TransactionOp::CreateProductType {
            symbol: DraftSymbol::generated(3),
            module: draft_symbol(2),
            name: "D".into(),
            fields: vec![
                ProductFieldDraft {
                    symbol: DraftSymbol::generated(4),
                    name: "same".into(),
                    ty: TypeDraft::I64,
                },
                ProductFieldDraft {
                    symbol: DraftSymbol::generated(5),
                    name: "same".into(),
                    ty: TypeDraft::Bool,
                },
            ],
        }],
    );
    assert_eq!(
        workspace
            .prepare_transaction(&duplicate)
            .expect_err("duplicate")
            .code,
        ErrorCode::DuplicateName
    );
    assert_eq!(workspace.head().expect("head").next_serial(), 2);
}

#[test]
fn nominal_operations_normalize_fields_and_match_arms_and_validate_payload_scope() {
    let id = WorkspaceId::from_bytes([0x94; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let operations = vec![
        TransactionOp::CreateProductType {
            symbol: DraftSymbol::generated(3),
            module: draft_symbol(2),
            name: "Pair".into(),
            fields: vec![
                ProductFieldDraft {
                    symbol: DraftSymbol::generated(4),
                    name: "left".into(),
                    ty: TypeDraft::I64,
                },
                ProductFieldDraft {
                    symbol: DraftSymbol::generated(5),
                    name: "right".into(),
                    ty: TypeDraft::I64,
                },
            ],
        },
        TransactionOp::CreateSumType {
            symbol: DraftSymbol::generated(6),
            module: draft_symbol(2),
            name: "Maybe".into(),
            variants: vec![
                SumVariantDraft {
                    symbol: DraftSymbol::generated(7),
                    name: "none".into(),
                    payload: None,
                },
                SumVariantDraft {
                    symbol: DraftSymbol::generated(8),
                    name: "some".into(),
                    payload: Some(TypeDraft::I64),
                },
            ],
        },
        TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(9),
            module: draft_symbol(2),
            name: "main".into(),
            parameters: Vec::new(),
            result: TypeDraft::I64,
            body: Some(FunctionBodyDraft {
                operations: vec![
                    draft_expression(20, ExpressionKindDraft::ConstI64(10)),
                    draft_expression(21, ExpressionKindDraft::ConstI64(20)),
                    draft_expression(
                        22,
                        ExpressionKindDraft::ConstructProduct {
                            product: draft_symbol(3),
                            fields: vec![
                                ProductFieldValueDraft {
                                    field: draft_symbol(5),
                                    value: draft_result(21),
                                },
                                ProductFieldValueDraft {
                                    field: draft_symbol(4),
                                    value: draft_result(20),
                                },
                            ],
                        },
                    ),
                    draft_expression(
                        23,
                        ExpressionKindDraft::ProjectField {
                            value: draft_result(22),
                            field: draft_symbol(4),
                        },
                    ),
                    draft_expression(
                        24,
                        ExpressionKindDraft::ConstructVariant {
                            variant: draft_symbol(8),
                            payload: Some(draft_result(23)),
                        },
                    ),
                    draft_expression(
                        25,
                        ExpressionKindDraft::MatchSum {
                            scrutinee: draft_result(24),
                            result: TypeDraft::I64,
                            arms: vec![
                                MatchArmDraft {
                                    variant: draft_symbol(8),
                                    payload_symbol: Some(DraftSymbol::generated(30)),
                                    body: YieldingBodyDraft {
                                        operations: Vec::new(),
                                        yield_value: ValueDraft::BlockArgument(draft_symbol(30)),
                                    },
                                },
                                MatchArmDraft {
                                    variant: draft_symbol(7),
                                    payload_symbol: None,
                                    body: YieldingBodyDraft {
                                        operations: vec![draft_expression(
                                            31,
                                            ExpressionKindDraft::ConstI64(0),
                                        )],
                                        yield_value: draft_result(31),
                                    },
                                },
                            ],
                        },
                    ),
                ],
                return_value: draft_result(25),
            }),
        },
    ];
    let request = structured_semantic_request(id, operations);
    let mutate_expression =
        |request: &mut ApplyTransactionRequest,
         symbol: u32,
         mutate: &mut dyn FnMut(&mut ExpressionKindDraft)| {
            let TransactionOp::CreateFunction {
                body: Some(body), ..
            } = request.transaction.operations.last_mut().expect("function")
            else {
                panic!("function")
            };
            let expression = body
                .operations
                .iter_mut()
                .find(|expression| expression.symbol == Some(DraftSymbol::generated(symbol)))
                .expect("expression");
            mutate(&mut expression.operation);
        };
    let mut invalid = request.clone();
    mutate_expression(&mut invalid, 22, &mut |operation| {
        let ExpressionKindDraft::ConstructProduct { fields, .. } = operation else {
            panic!("product")
        };
        fields.pop();
    });
    assert_eq!(
        workspace
            .prepare_transaction(&invalid)
            .expect_err("missing field")
            .code,
        ErrorCode::InvalidOperand
    );
    let mut invalid = request.clone();
    mutate_expression(&mut invalid, 22, &mut |operation| {
        let ExpressionKindDraft::ConstructProduct { fields, .. } = operation else {
            panic!("product")
        };
        fields.push(fields[0].clone());
    });
    assert_eq!(
        workspace
            .prepare_transaction(&invalid)
            .expect_err("duplicate field")
            .code,
        ErrorCode::InvalidOperand
    );
    let mut invalid = request.clone();
    mutate_expression(&mut invalid, 20, &mut |operation| {
        *operation = ExpressionKindDraft::ConstBool(true)
    });
    assert_eq!(
        workspace
            .prepare_transaction(&invalid)
            .expect_err("wrong field type")
            .code,
        ErrorCode::TypeMismatch
    );
    let mut invalid = request.clone();
    mutate_expression(&mut invalid, 25, &mut |operation| {
        let ExpressionKindDraft::MatchSum { arms, .. } = operation else {
            panic!("match")
        };
        arms.pop();
    });
    assert_eq!(
        workspace
            .prepare_transaction(&invalid)
            .expect_err("missing arm")
            .code,
        ErrorCode::InvalidOperand
    );
    let mut invalid = request.clone();
    mutate_expression(&mut invalid, 25, &mut |operation| {
        let ExpressionKindDraft::MatchSum { arms, .. } = operation else {
            panic!("match")
        };
        arms[0].payload_symbol = None;
    });
    assert_eq!(
        workspace
            .prepare_transaction(&invalid)
            .expect_err("missing payload binding")
            .code,
        ErrorCode::InvalidDraftSymbol
    );
    assert_eq!(workspace.head().expect("head").next_serial(), 2);
    let prepared = workspace
        .prepare_transaction(&request)
        .expect("nominal operations");
    let product = prepared
        .snapshot
        .nodes()
        .find_map(|(operation_id, node)| match node {
            Node::Operation {
                operation: OperationKind::ConstructProduct { product, fields },
                ..
            } => Some((operation_id, *product, fields.clone())),
            _ => None,
        })
        .expect("product operation");
    let Node::ProductType {
        fields: declared, ..
    } = prepared.snapshot.node(product.1).expect("product")
    else {
        unreachable!()
    };
    assert_eq!(
        product
            .2
            .iter()
            .map(|binding| binding.field)
            .collect::<Vec<_>>(),
        *declared
    );
    let second_field_context = crate::query::execute(
        &prepared.snapshot,
        &crate::query::Query::RepairContext {
            target: crate::query::RepairTarget::Operand {
                operation: product.0,
                index: 1,
            },
            budget: crate::query::ContextBudget {
                body_before: 0,
                body_after: 0,
                visible_values: 1,
                incoming_uses: 1,
                include_incompatible: false,
            },
        },
        None,
    )
    .expect("second product field context");
    let crate::query::QueryResult::RepairContext(second_field_context) = second_field_context
    else {
        panic!("repair context")
    };
    assert_eq!(
        second_field_context.use_mode,
        Some(crate::schema::OperandUse::Read)
    );

    let arms = prepared
        .snapshot
        .nodes()
        .find_map(|(_, node)| match node {
            Node::Operation {
                operation: OperationKind::MatchSum { arms, .. },
                ..
            } => Some(arms.clone()),
            _ => None,
        })
        .expect("match operation");
    let sum = match prepared.snapshot.node(arms[0].variant).expect("variant") {
        Node::SumVariant { owner, .. } => *owner,
        _ => unreachable!(),
    };
    let Node::SumType { variants, .. } = prepared.snapshot.node(sum).expect("sum") else {
        unreachable!()
    };
    assert_eq!(
        arms.iter().map(|arm| arm.variant).collect::<Vec<_>>(),
        *variants
    );
    workspace.publish(prepared.snapshot).expect("publish");
}

#[test]
fn nominal_hole_refinement_is_atomic_and_preserves_identity() {
    let id = WorkspaceId::from_bytes([0x95; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let request = structured_semantic_request(
        id,
        vec![
            TransactionOp::CreateProductType {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "Pair".into(),
                fields: vec![
                    ProductFieldDraft {
                        symbol: DraftSymbol::generated(4),
                        name: "left".into(),
                        ty: TypeDraft::I64,
                    },
                    ProductFieldDraft {
                        symbol: DraftSymbol::generated(5),
                        name: "right".into(),
                        ty: TypeDraft::I64,
                    },
                ],
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(6),
                module: draft_symbol(2),
                name: "make".into(),
                parameters: Vec::new(),
                result: TypeDraft::Nominal(draft_symbol(3)),
                body: Some(FunctionBodyDraft {
                    operations: vec![
                        draft_expression(20, ExpressionKindDraft::ConstI64(1)),
                        draft_expression(21, ExpressionKindDraft::ConstI64(2)),
                        draft_expression(
                            22,
                            ExpressionKindDraft::Hole {
                                expected: TypeDraft::Nominal(draft_symbol(3)),
                            },
                        ),
                    ],
                    return_value: draft_result(22),
                }),
            },
        ],
    );
    let prepared = workspace
        .prepare_transaction(&request)
        .expect("incomplete product function");
    let prior = prepared.snapshot.clone();
    workspace
        .publish(prepared.snapshot)
        .expect("publish incomplete");
    let product = prior
        .nodes()
        .find_map(|(id, node)| matches!(node, Node::ProductType { .. }).then_some(id))
        .expect("product");
    let fields = match prior.node(product).expect("product") {
        Node::ProductType { fields, .. } => fields.clone(),
        _ => unreachable!(),
    };
    let hole = prior
        .nodes()
        .find_map(|(id, node)| {
            matches!(
                node,
                Node::Operation {
                    operation: OperationKind::Hole { .. },
                    ..
                }
            )
            .then_some(id)
        })
        .expect("hole");
    let values = prior
        .nodes()
        .filter_map(|(id, node)| {
            matches!(
                node,
                Node::Operation {
                    operation: OperationKind::ConstI64(_),
                    ..
                }
            )
            .then_some(id)
        })
        .collect::<Vec<_>>();
    let field_value = |field: NodeId, value: NodeId| ProductFieldValueDraft {
        field: NodeTarget::Existing(field),
        value: ValueDraft::OperationResult {
            operation: NodeTarget::Existing(value),
            output: 0,
        },
    };
    let first_page = crate::query::execute(
        &prior,
        &crate::query::Query::NominalType {
            declaration: product,
            page: crate::query::PageRequest {
                after: None,
                limit: 1,
            },
        },
        None,
    )
    .expect("nominal page");
    let crate::query::QueryResult::NominalType(first_page) = first_page else {
        panic!("nominal page")
    };
    assert_eq!(first_page.members.items.len(), 1);
    assert_eq!(first_page.members.total, Some(2));
    assert!(first_page.layout.representable);
    let cursor = first_page.members.next.expect("nominal continuation");
    let second_page = crate::query::execute(
        &prior,
        &crate::query::Query::NominalType {
            declaration: product,
            page: crate::query::PageRequest {
                after: Some(cursor),
                limit: 1,
            },
        },
        None,
    )
    .expect("nominal continuation");
    let crate::query::QueryResult::NominalType(second_page) = second_page else {
        panic!("nominal page")
    };
    assert_eq!(second_page.members.items.len(), 1);
    assert!(second_page.members.next.is_none());
    let context = crate::query::execute(
        &prior,
        &crate::query::Query::RepairContext {
            target: crate::query::RepairTarget::Hole(hole),
            budget: crate::query::ContextBudget {
                body_before: 1,
                body_after: 1,
                visible_values: 8,
                incoming_uses: 8,
                include_incompatible: false,
            },
        },
        None,
    )
    .expect("nominal repair context");
    let crate::query::QueryResult::RepairContext(context) = context else {
        panic!("repair context")
    };
    assert_eq!(
        context
            .nominal_type
            .as_ref()
            .and_then(|nominal| nominal.members.total),
        Some(2)
    );
    assert!(
        context
            .legal_constructors
            .iter()
            .any(
                |constructor| constructor.code == crate::schema::OperationCode::ConstructProduct
                    && constructor.members == fields
            )
    );
    let invalid = ApplyTransactionRequest {
        transaction: Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::ConstructProduct {
                    product: NodeTarget::Existing(product),
                    fields: vec![field_value(fields[0], values[0])],
                },
            }],
        },
        response: TransactionResponseSpec::default(),
    };
    assert_eq!(
        workspace
            .prepare_transaction(&invalid)
            .expect_err("missing field")
            .code,
        ErrorCode::InvalidOperand
    );
    assert_eq!(workspace.head_revision(), Revision::new(1));
    let valid = ApplyTransactionRequest {
        transaction: Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::ConstructProduct {
                    product: NodeTarget::Existing(product),
                    fields: vec![
                        field_value(fields[1], values[1]),
                        field_value(fields[0], values[0]),
                    ],
                },
            }],
        },
        response: TransactionResponseSpec::default(),
    };
    let mut validate_only = valid.clone();
    validate_only.transaction.mode = TransactionMode::ValidateOnly;
    let predicted = workspace
        .prepare_transaction(&validate_only)
        .expect("validate-only product refinement");
    assert!(!predicted.receipt.published);
    assert_eq!(workspace.head_revision(), Revision::new(1));
    let prepared = workspace
        .prepare_transaction(&valid)
        .expect("valid product refinement");
    assert!(matches!(
        prepared.snapshot.node(hole),
        Ok(Node::Operation {
            operation: OperationKind::ConstructProduct { .. },
            ..
        })
    ));
    let changes = crate::diff::between(&prior, &prepared.snapshot);
    assert!(changes.changes.iter().any(|change| change.node == hole
        && matches!(
            change.kind,
            crate::diff::ChangeKind::OperationRefined {
                after: crate::schema::OperationCode::ConstructProduct,
                ..
            }
        )));
    workspace
        .publish(prepared.snapshot)
        .expect("publish refinement");
}

#[test]
fn nominal_type_references_block_declaration_deletion() {
    let id = WorkspaceId::from_bytes([0x93; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let mut transaction = structured_semantic_request(
        id,
        vec![
            TransactionOp::CreateProductType {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "Reading".into(),
                fields: vec![ProductFieldDraft {
                    symbol: DraftSymbol::generated(4),
                    name: "value".into(),
                    ty: TypeDraft::I64,
                }],
            },
            TransactionOp::CreateSumType {
                symbol: DraftSymbol::generated(5),
                module: draft_symbol(2),
                name: "Input".into(),
                variants: vec![SumVariantDraft {
                    symbol: DraftSymbol::generated(6),
                    name: "sample".into(),
                    payload: Some(TypeDraft::Nominal(draft_symbol(3))),
                }],
            },
        ],
    );
    transaction.response.return_symbols = vec![DraftSymbol::generated(3)];
    let prepared = workspace
        .prepare_transaction(&transaction)
        .expect("declarations");
    let reading = prepared
        .receipt
        .returned_bindings
        .iter()
        .find(|(symbol, _)| *symbol == DraftSymbol::generated(3))
        .expect("reading binding")
        .1;
    workspace.publish(prepared.snapshot).expect("publish");
    let delete = Transaction {
        workspace: id,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::DeleteOwnedSubtree {
            root: NodeTarget::Existing(reading),
        }],
    };
    assert_eq!(
        workspace
            .prepare_transaction(&request(&delete))
            .expect_err("referenced declaration")
            .code,
        ErrorCode::DeleteBlocked
    );
    assert_eq!(workspace.head_revision(), Revision::new(1));
}

#[test]
fn stale_revisions_wrong_workspaces_and_no_changes_reject_deterministically() {
    let id = WorkspaceId::from_bytes([13; 16]);
    let other = WorkspaceId::from_bytes([14; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let first = commit(&mut workspace, &create_package_and_module(id)).expect("first commit");
    let package = first.returned_bindings[0].1;

    let stale = create_package_and_module(id);
    assert_eq!(
        workspace
            .prepare_transaction(&request(&stale))
            .expect_err("stale")
            .code,
        ErrorCode::RevisionConflict
    );
    let wrong = Transaction {
        workspace: id,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::RenameNode {
            node: NodeTarget::Existing(NodeId::new(other, package.serial()).expect("node")),
            name: "renamed".to_owned(),
        }],
    };
    assert_eq!(
        workspace
            .prepare_transaction(&request(&wrong))
            .expect_err("wrong workspace")
            .code,
        ErrorCode::WrongWorkspace
    );
    let no_change = Transaction {
        workspace: id,
        base_revision: Revision::new(1),
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::RenameNode {
            node: NodeTarget::Existing(package),
            name: "package".to_owned(),
        }],
    };
    assert_eq!(
        workspace
            .prepare_transaction(&request(&no_change))
            .expect_err("no change")
            .code,
        ErrorCode::NoChange
    );
}

#[test]
fn preallocation_scan_covers_top_level_types_values_and_maintenance_targets() {
    let local = |value| NodeTarget::Draft(DraftSymbol::generated(value));
    let cases = vec![
        vec![TransactionOp::CreateModule {
            symbol: DraftSymbol::generated(1),
            package: local(99),
            name: "m".into(),
        }],
        vec![
            TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(1),
                name: "p".into(),
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(2),
                module: local(99),
                name: "f".into(),
                parameters: Vec::new(),
                result: TypeDraft::Nominal(local(98)),
                body: None,
            },
        ],
        vec![TransactionOp::ReplaceOperand {
            operation: local(97),
            index: 0,
            value: ValueDraft::OperationResult {
                operation: local(96),
                output: 0,
            },
        }],
        vec![TransactionOp::RenameNode {
            node: local(95),
            name: "renamed".into(),
        }],
    ];
    for operations in cases {
        assert_eq!(
            validate_structured_request(&operations)
                .expect_err("undeclared scan path")
                .code,
            ErrorCode::InvalidDraftSymbol
        );
    }

    let wrong_kind = vec![
        TransactionOp::CreatePackage {
            symbol: DraftSymbol::generated(1),
            name: "p".into(),
        },
        TransactionOp::CreateModule {
            symbol: DraftSymbol::generated(2),
            package: local(1),
            name: "m".into(),
        },
        TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(3),
            module: local(2),
            name: "f".into(),
            parameters: Vec::new(),
            result: TypeDraft::I64,
            body: Some(FunctionBodyDraft {
                operations: vec![draft_expression(
                    4,
                    ExpressionKindDraft::Call {
                        function: local(1),
                        arguments: Vec::new(),
                    },
                )],
                return_value: draft_result(4),
            }),
        },
    ];
    let error = validate_structured_request(&wrong_kind).expect_err("wrong local category");
    assert_eq!(error.code, ErrorCode::WrongKind);
    assert_eq!(error.draft_symbol, Some(DraftSymbol::generated(1)));
}

#[test]
fn preallocation_scan_rejects_non_region_if_and_for_targets() {
    let prefix = || {
        vec![
            TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(1),
                name: "p".into(),
            },
            TransactionOp::CreateModule {
                symbol: DraftSymbol::generated(2),
                package: draft_symbol(1),
                name: "m".into(),
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "f".into(),
                parameters: Vec::new(),
                result: TypeDraft::I64,
                body: Some(FunctionBodyDraft {
                    operations: vec![draft_expression(4, ExpressionKindDraft::ConstI64(0))],
                    return_value: draft_result(4),
                }),
            },
        ]
    };
    let value = draft_result(4);
    let mut if_target = prefix();
    if_target.push(TransactionOp::ReplaceOperation {
        operation: draft_symbol(4),
        replacement: OperationDraft::If {
            condition: value.clone(),
            result: TypeDraft::I64,
            then_region: draft_symbol(3),
            else_region: draft_symbol(3),
        },
    });
    let mut for_target = prefix();
    for_target.push(TransactionOp::ReplaceOperation {
        operation: draft_symbol(4),
        replacement: OperationDraft::ForI64 {
            start: value.clone(),
            end_exclusive: value.clone(),
            step: 1,
            initial: value.clone(),
            carried: TypeDraft::I64,
            body_region: draft_symbol(3),
        },
    });

    for operations in [if_target, for_target] {
        let error = scan_explicit_symbols(&operations)
            .expect_err("non-region target must reject during the preallocation scan");
        assert_eq!(error.code, ErrorCode::WrongKind);
        assert_eq!(error.operation_index, Some(3));
        assert_eq!(error.draft_symbol, Some(DraftSymbol::generated(3)));
    }
}

#[test]
fn later_nominal_declarations_and_permuted_match_arms_expand_identically() {
    let id = WorkspaceId::from_bytes([0xa4; 16]);
    let make = |permuted: bool| {
        let none = MatchArmDraft {
            variant: draft_symbol(11),
            payload_symbol: None,
            body: YieldingBodyDraft {
                operations: vec![draft_expression(30, ExpressionKindDraft::ConstI64(0))],
                yield_value: draft_result(30),
            },
        };
        let some = MatchArmDraft {
            variant: draft_symbol(12),
            payload_symbol: Some(DraftSymbol::generated(31)),
            body: YieldingBodyDraft {
                operations: Vec::new(),
                yield_value: ValueDraft::BlockArgument(draft_symbol(31)),
            },
        };
        let arms = if permuted {
            vec![some.clone(), none.clone()]
        } else {
            vec![none, some]
        };
        let fields = if permuted {
            vec![
                ProductFieldValueDraft {
                    field: draft_symbol(6),
                    value: draft_result(20),
                },
                ProductFieldValueDraft {
                    field: draft_symbol(5),
                    value: draft_result(20),
                },
            ]
        } else {
            vec![
                ProductFieldValueDraft {
                    field: draft_symbol(5),
                    value: draft_result(20),
                },
                ProductFieldValueDraft {
                    field: draft_symbol(6),
                    value: draft_result(20),
                },
            ]
        };
        ApplyTransactionRequest {
            transaction: Transaction {
                workspace: id,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::ValidateOnly,
                operations: vec![
                    TransactionOp::CreatePackage {
                        symbol: DraftSymbol::generated(1),
                        name: "p".into(),
                    },
                    TransactionOp::CreateModule {
                        symbol: DraftSymbol::generated(2),
                        package: draft_symbol(1),
                        name: "m".into(),
                    },
                    TransactionOp::CreateFunction {
                        symbol: DraftSymbol::generated(3),
                        module: draft_symbol(2),
                        name: "forward".into(),
                        parameters: Vec::new(),
                        result: TypeDraft::I64,
                        body: Some(FunctionBodyDraft {
                            operations: vec![
                                draft_expression(20, ExpressionKindDraft::ConstI64(7)),
                                draft_expression(
                                    21,
                                    ExpressionKindDraft::ConstructProduct {
                                        product: draft_symbol(4),
                                        fields,
                                    },
                                ),
                                draft_expression(
                                    22,
                                    ExpressionKindDraft::ProjectField {
                                        value: draft_result(21),
                                        field: draft_symbol(5),
                                    },
                                ),
                                draft_expression(
                                    23,
                                    ExpressionKindDraft::ConstructVariant {
                                        variant: draft_symbol(12),
                                        payload: Some(draft_result(22)),
                                    },
                                ),
                                draft_expression(
                                    24,
                                    ExpressionKindDraft::MatchSum {
                                        scrutinee: draft_result(23),
                                        result: TypeDraft::I64,
                                        arms,
                                    },
                                ),
                            ],
                            return_value: draft_result(24),
                        }),
                    },
                    TransactionOp::CreateProductType {
                        symbol: DraftSymbol::generated(4),
                        module: draft_symbol(2),
                        name: "Pair".into(),
                        fields: vec![
                            ProductFieldDraft {
                                symbol: DraftSymbol::generated(5),
                                name: "left".into(),
                                ty: TypeDraft::I64,
                            },
                            ProductFieldDraft {
                                symbol: DraftSymbol::generated(6),
                                name: "right".into(),
                                ty: TypeDraft::I64,
                            },
                        ],
                    },
                    TransactionOp::CreateSumType {
                        symbol: DraftSymbol::generated(10),
                        module: draft_symbol(2),
                        name: "Maybe".into(),
                        variants: vec![
                            SumVariantDraft {
                                symbol: DraftSymbol::generated(11),
                                name: "none".into(),
                                payload: None,
                            },
                            SumVariantDraft {
                                symbol: DraftSymbol::generated(12),
                                name: "some".into(),
                                payload: Some(TypeDraft::I64),
                            },
                        ],
                    },
                ],
            },
            response: TransactionResponseSpec::default(),
        }
    };
    let workspace = Workspace::new(id).expect("workspace");
    let canonical = workspace
        .prepare_transaction(&make(false))
        .expect("canonical arms");
    let permuted = workspace
        .prepare_transaction(&make(true))
        .expect("permuted arms");
    assert_eq!(canonical.snapshot.hash(), permuted.snapshot.hash());
    assert_eq!(canonical.snapshot.nodes, permuted.snapshot.nodes);
}

#[test]
fn draft_symbol_spelling_does_not_change_allocation_graph_or_execution() {
    let id = WorkspaceId::from_bytes([0xd1; 16]);
    let request = |names: [&str; 6]| {
        let symbols = names.map(DraftSymbol::new);
        let [package, module, function, one, two, sum] = symbols;
        ApplyTransactionRequest {
            transaction: Transaction {
                workspace: id,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::ValidateOnly,
                operations: vec![
                    TransactionOp::CreatePackage {
                        symbol: package,
                        name: "package".into(),
                    },
                    TransactionOp::CreateModule {
                        symbol: module,
                        package: NodeTarget::Draft(package),
                        name: "module".into(),
                    },
                    TransactionOp::CreateFunction {
                        symbol: function,
                        module: NodeTarget::Draft(module),
                        name: "main".into(),
                        parameters: Vec::new(),
                        result: TypeDraft::I64,
                        body: Some(FunctionBodyDraft {
                            operations: vec![
                                ExpressionDraft {
                                    symbol: Some(one),
                                    operation: ExpressionKindDraft::ConstI64(1),
                                },
                                ExpressionDraft {
                                    symbol: Some(two),
                                    operation: ExpressionKindDraft::ConstI64(2),
                                },
                                ExpressionDraft {
                                    symbol: Some(sum),
                                    operation: ExpressionKindDraft::AddI64 {
                                        lhs: ValueDraft::OperationResult {
                                            operation: NodeTarget::Draft(one),
                                            output: 0,
                                        },
                                        rhs: ValueDraft::OperationResult {
                                            operation: NodeTarget::Draft(two),
                                            output: 0,
                                        },
                                    },
                                },
                            ],
                            return_value: ValueDraft::OperationResult {
                                operation: NodeTarget::Draft(sum),
                                output: 0,
                            },
                        }),
                    },
                    TransactionOp::SetEntryFunction {
                        package: NodeTarget::Draft(package),
                        function: NodeTarget::Draft(function),
                    },
                ],
            },
            response: TransactionResponseSpec {
                return_symbols: vec![function],
            },
        }
    };
    let first = Workspace::new(id)
        .expect("first workspace")
        .prepare_transaction(&request(["package", "module", "main", "one", "two", "sum"]))
        .expect("first proposal");
    let renamed = Workspace::new(id)
        .expect("renamed workspace")
        .prepare_transaction(&request(["p", "m", "entry", "a", "b", "answer"]))
        .expect("renamed proposal");
    assert_eq!(first.snapshot.nodes, renamed.snapshot.nodes);
    assert_eq!(first.snapshot.hash(), renamed.snapshot.hash());
    let entry = first.receipt.returned_bindings[0].1;
    assert_eq!(entry, renamed.receipt.returned_bindings[0].1);
    for prepared in [&first, &renamed] {
        let run = crate::interpret::compile_and_run(
            &prepared.snapshot,
            entry,
            &[],
            crate::interpret::RunPolicy {
                fuel: 100,
                maximum_frames: 16,
            },
        )
        .expect("canonical execution");
        assert_eq!(run.value, crate::interpret::RuntimeValue::I64(3));
    }
}

#[test]
fn product_second_operand_is_read_and_oversized_constructor_requirements_are_bounded() {
    let id = WorkspaceId::from_bytes([0xa5; 16]);
    let workspace = Workspace::new(id).expect("workspace");
    let fields = (0..65)
        .map(|index| ProductFieldDraft {
            symbol: DraftSymbol::generated(100 + index),
            name: format!("field_{index}"),
            ty: TypeDraft::I64,
        })
        .collect::<Vec<_>>();
    let parameters = (0..65)
        .map(|index| FunctionParameterDraft {
            symbol: DraftSymbol::generated(300 + index),
            name: format!("parameter_{index}"),
            ty: TypeDraft::I64,
        })
        .collect::<Vec<_>>();
    let request = structured_semantic_request(
        id,
        vec![
            TransactionOp::CreateProductType {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "Wide".into(),
                fields,
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(4),
                module: draft_symbol(2),
                name: "wide_call".into(),
                parameters,
                result: TypeDraft::Nominal(draft_symbol(3)),
                body: None,
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(5),
                module: draft_symbol(2),
                name: "repair".into(),
                parameters: Vec::new(),
                result: TypeDraft::Nominal(draft_symbol(3)),
                body: Some(FunctionBodyDraft {
                    operations: vec![draft_expression(
                        6,
                        ExpressionKindDraft::Hole {
                            expected: TypeDraft::Nominal(draft_symbol(3)),
                        },
                    )],
                    return_value: draft_result(6),
                }),
            },
        ],
    );
    let prepared = workspace
        .prepare_transaction(&request)
        .expect("wide product");
    let hole = prepared
        .snapshot
        .nodes()
        .find_map(|(id, node)| {
            matches!(
                node,
                Node::Operation {
                    operation: OperationKind::Hole { .. },
                    ..
                }
            )
            .then_some(id)
        })
        .expect("hole");
    let context = crate::query::execute(
        &prepared.snapshot,
        &crate::query::Query::RepairContext {
            target: crate::query::RepairTarget::Hole(hole),
            budget: crate::query::ContextBudget {
                body_before: 1,
                body_after: 1,
                visible_values: 1,
                incoming_uses: 1,
                include_incompatible: false,
            },
        },
        None,
    )
    .expect("context");
    let crate::query::QueryResult::RepairContext(context) = context else {
        panic!("context")
    };
    assert!(context.nominal_type.is_none());
    assert!(context.nominal_type_continuation.is_some());
    for constructor in &context.legal_constructors {
        assert!(constructor.operand_types.len() <= crate::query::MAX_CONTEXT_ITEMS as usize);
        assert!(constructor.operand_uses.len() <= crate::query::MAX_CONTEXT_ITEMS as usize);
        assert!(constructor.members.len() <= crate::query::MAX_CONTEXT_ITEMS as usize);
    }
    let product = context
        .legal_constructors
        .iter()
        .find(|constructor| constructor.code == crate::schema::OperationCode::ConstructProduct)
        .expect("product constructor");
    assert_eq!(product.operand_count, 65);
    assert_eq!(product.member_count, 65);
    assert!(!product.requirements_complete);
    assert!(product.nominal_type_continuation.is_some());
    let call = context
        .legal_constructors
        .iter()
        .find(|constructor| constructor.code == crate::schema::OperationCode::Call)
        .expect("call constructor");
    assert_eq!(call.operand_count, 65);
    assert!(!call.requirements_complete);
}
