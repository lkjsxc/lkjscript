use crate::artifact;
use crate::diff;
use crate::error::ErrorCode;
use crate::graph::{Snapshot, Workspace};
use crate::ids::{DraftSymbol, IdempotencyKey, NodeId, QueryId, RequestId, Revision, WorkspaceId};
use crate::machine::{self, RequestEnvelope};
use crate::persistence::{self, DurableWorkspace};
use crate::protocol::Request;
use crate::query::{
    ContextBudget, PageRequest, Query, QueryBatchRequest, QueryItem, QueryResult, RepairTarget,
    VisibleCursorPurpose,
};
use crate::schema::{OperationDraft, SemanticType, TypeDraft, ValueDraft, ValueRef};
use crate::transaction::{
    ApplyTransactionRequest, ExpressionDraft, ExpressionKindDraft, FunctionBodyDraft,
    FunctionParameterDraft, NodeTarget, SumVariantDraft, Transaction, TransactionMode,
    TransactionOp, TransactionReceipt, TransactionResponseSpec, YieldingBodyDraft,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::sync::Arc;

#[derive(Clone, Copy)]
struct Prng(u64);

impl Prng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn index(&mut self, length: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(length).expect("bounded length"))
            .expect("bounded index")
    }
}

fn local(symbol: u32) -> NodeTarget {
    NodeTarget::Draft(DraftSymbol::generated(symbol))
}

fn existing(id: NodeId) -> NodeTarget {
    NodeTarget::Existing(id)
}

fn local_value(symbol: u32) -> ValueDraft {
    ValueDraft::OperationResult {
        operation: local(symbol),
        output: 0,
    }
}

fn existing_value(operation: NodeId, output: u8) -> ValueDraft {
    ValueDraft::OperationResult {
        operation: existing(operation),
        output,
    }
}

fn request(transaction: Transaction, selected: &[u32]) -> ApplyTransactionRequest {
    ApplyTransactionRequest {
        transaction,
        response: TransactionResponseSpec {
            return_symbols: selected
                .iter()
                .copied()
                .map(DraftSymbol::generated)
                .collect(),
        },
    }
}

fn fixture(workspace: WorkspaceId, seed: u64, mode: TransactionMode) -> ApplyTransactionRequest {
    let suffix = seed % 10_000;
    request(
        Transaction {
            workspace,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: format!("package-{suffix}"),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: local(1),
                    name: "module".to_owned(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: local(2),
                    name: "main".to_owned(),
                    parameters: vec![FunctionParameterDraft {
                        symbol: DraftSymbol::generated(4),
                        name: "input".to_owned(),
                        ty: SemanticType::I64.into(),
                    }],
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(7)),
                                operation: ExpressionKindDraft::ConstI64(
                                    40 + i64::try_from(seed % 3).expect("small"),
                                ),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(8)),
                                operation: ExpressionKindDraft::ConstI64(2),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(9)),
                                operation: ExpressionKindDraft::ConstBool(true),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(10)),
                                operation: ExpressionKindDraft::Hole {
                                    expected: SemanticType::I64.into(),
                                },
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(11)),
                                operation: ExpressionKindDraft::ConstI64(99),
                            },
                        ],
                        return_value: local_value(10),
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: local(1),
                    function: local(3),
                },
            ],
        },
        &[1, 2, 3, 4, 7, 8, 9, 10, 11],
    )
}

fn binding(receipt: &TransactionReceipt, symbol: u32) -> NodeId {
    receipt
        .returned_bindings
        .iter()
        .find_map(|(candidate, id)| (candidate.generated_number() == symbol).then_some(*id))
        .expect("selected binding")
}

fn assert_snapshot_invariants(snapshot: &Snapshot) {
    crate::validate::validate_snapshot(snapshot).expect("production snapshot validator");
    let bytes = artifact::encode(snapshot).expect("canonical artifact encode");
    let decoded = artifact::decode(&bytes).expect("canonical artifact decode");
    assert_eq!(decoded, *snapshot);
    assert_eq!(
        artifact::encode(&decoded).expect("canonical re-encode"),
        bytes
    );
    let live: BTreeSet<u64> = snapshot.nodes().map(|(id, _)| id.serial()).collect();
    let tombstones: BTreeSet<u64> = snapshot.tombstones().collect();
    assert!(live.is_disjoint(&tombstones));
    for serial in 1..snapshot.next_serial() {
        assert!(
            live.contains(&serial) || tombstones.contains(&serial),
            "allocated serial {serial} is neither live nor tombstoned"
        );
    }
}

fn commit_checked(
    workspace: &mut Workspace,
    request: &ApplyTransactionRequest,
) -> TransactionReceipt {
    let before = workspace.head().expect("head").clone();
    let retained_before: Vec<_> = (0..=workspace.head_revision().get())
        .map(|revision| {
            let revision = Revision::new(revision);
            (
                revision,
                artifact::encode(workspace.snapshot(revision).expect("retained snapshot"))
                    .expect("retained artifact"),
            )
        })
        .collect();
    let before_revision = workspace.head_revision();
    let before_next = before.next_serial();
    let prepared = workspace
        .prepare_transaction(request)
        .expect("generated accepted action");
    assert_eq!(
        prepared.snapshot.revision(),
        before_revision.next().expect("next revision")
    );
    assert_eq!(
        prepared.snapshot.next_serial(),
        before_next + prepared.receipt.created_count
    );
    assert_snapshot_invariants(&prepared.snapshot);
    let semantic_diff = diff::between(&before, &prepared.snapshot);
    assert_eq!(prepared.receipt.change_count, semantic_diff.change_count());
    assert_eq!(prepared.receipt.change_digest, semantic_diff.digest);
    let mut cursor = None;
    let mut queried_changes = Vec::new();
    loop {
        let QueryResult::SemanticDiff(queried_diff) = crate::query::execute(
            &prepared.snapshot,
            &Query::SemanticDiff {
                from: before.revision(),
                page: PageRequest {
                    after: cursor,
                    limit: 1,
                },
            },
            Some(&before),
        )
        .expect("snapshot-derived paginated diff query") else {
            panic!("semantic diff query result")
        };
        assert_eq!(queried_diff.change_count, prepared.receipt.change_count);
        assert_eq!(queried_diff.change_digest, prepared.receipt.change_digest);
        queried_changes.extend(queried_diff.page.items);
        cursor = queried_diff.page.next;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(queried_changes, semantic_diff.changes);
    assert_eq!(
        prepared
            .receipt
            .returned_bindings
            .iter()
            .map(|(symbol, _)| *symbol)
            .collect::<Vec<_>>(),
        request.response.return_symbols
    );
    for (_, node) in &prepared.receipt.returned_bindings {
        if node.is_durable() {
            assert!(node.serial() >= before_next);
            assert!(node.serial() < prepared.snapshot.next_serial());
        } else {
            assert!(node.is_function_local());
            assert!(node.local_ordinal().is_some());
        }
        assert!(
            prepared.snapshot.node(*node).is_ok()
                || (node.is_durable() && prepared.snapshot.contains_tombstone(node.serial()))
        );
    }
    let receipt = prepared.receipt.clone();
    workspace
        .publish(prepared.snapshot)
        .expect("publish accepted snapshot");
    assert_eq!(
        workspace.head_revision(),
        before_revision.next().expect("next revision")
    );
    for (revision, bytes) in retained_before {
        assert_eq!(
            artifact::encode(workspace.snapshot(revision).expect("retained snapshot"))
                .expect("retained old artifact"),
            bytes
        );
    }
    assert_snapshot_invariants(workspace.head().expect("published head"));
    let snapshots: BTreeMap<_, _> = (0..=workspace.head_revision().get())
        .map(|revision| {
            let revision = Revision::new(revision);
            (
                revision,
                Arc::new(
                    artifact::decode(
                        &artifact::encode(workspace.snapshot(revision).expect("history snapshot"))
                            .expect("history artifact"),
                    )
                    .expect("history decode"),
                ),
            )
        })
        .collect();
    Workspace::from_snapshots(workspace.id(), workspace.head_revision(), snapshots)
        .expect("complete retained history reconstructs");
    receipt
}

fn predict_next(workspace: &Workspace, name: &str) -> NodeId {
    let prediction = request(
        Transaction {
            workspace: workspace.id(),
            base_revision: workspace.head_revision(),
            idempotency_key: None,
            mode: TransactionMode::ValidateOnly,
            operations: vec![TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(60_000),
                name: name.to_owned(),
            }],
        },
        &[60_000],
    );
    let prepared = workspace
        .prepare_transaction(&prediction)
        .expect("next allocation prediction");
    assert!(!prepared.receipt.published);
    binding(&prepared.receipt, 60_000)
}

fn reject_checked(
    workspace: &Workspace,
    request: &ApplyTransactionRequest,
    prediction_name: &str,
    expected: ErrorCode,
) {
    let before = workspace.head().expect("head");
    let revision = before.revision();
    let hash = before.hash();
    let next_serial = before.next_serial();
    let tombstones: Vec<_> = before.tombstones().collect();
    let bytes = artifact::encode(before).expect("before artifact");
    let predicted = predict_next(workspace, prediction_name);
    let error = workspace
        .prepare_transaction(request)
        .expect_err("generated invalid transaction must reject");
    assert_eq!(error.code, expected);
    let after = workspace.head().expect("head after rejection");
    assert_eq!(after.revision(), revision);
    assert_eq!(after.hash(), hash);
    assert_eq!(after.next_serial(), next_serial);
    assert_eq!(after.tombstones().collect::<Vec<_>>(), tombstones);
    assert_eq!(artifact::encode(after).expect("after artifact"), bytes);
    assert_eq!(predict_next(workspace, prediction_name), predicted);
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Action {
    Rename,
    ScalarEdit,
    InvalidRefinement,
    InvalidType,
    InvalidOrder,
    InvalidOutput,
    Refine,
    OperandEdit,
    CreateDelete,
    ValidateThenCommit,
    StaleRevision,
    WrongWorkspace,
    DuplicateDraftSymbol,
    DuplicateName,
    InvalidSelected,
    StructuredScenario,
}

impl Action {
    const ALL: [Self; 16] = [
        Self::Rename,
        Self::ScalarEdit,
        Self::InvalidRefinement,
        Self::InvalidType,
        Self::InvalidOrder,
        Self::InvalidOutput,
        Self::Refine,
        Self::OperandEdit,
        Self::CreateDelete,
        Self::ValidateThenCommit,
        Self::StaleRevision,
        Self::WrongWorkspace,
        Self::DuplicateDraftSymbol,
        Self::DuplicateName,
        Self::InvalidSelected,
        Self::StructuredScenario,
    ];
}

#[test]
fn deterministic_generated_transaction_sequences() {
    const SEEDS: [u64; 5] = [1, 0x5eed, 0xdecafbad, 0x9e37_79b9, u32::MAX as u64];
    for seed in SEEDS {
        let mut trace = Vec::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            generated_sequence(seed, &mut trace);
        }));
        if let Err(payload) = result {
            eprintln!("generated transaction sequence failed: seed={seed} trace={trace:?}");
            std::panic::resume_unwind(payload);
        }
    }
}

fn generated_sequence(seed: u64, trace: &mut Vec<Action>) {
    let mut random = Prng::new(seed);
    let workspace_id =
        WorkspaceId::from_bytes(seed.to_le_bytes().repeat(2).try_into().expect("ID"));
    let other = WorkspaceId::from_bytes([0xee; 16]);
    let mut workspace = Workspace::new(workspace_id).expect("workspace");
    let predicted = workspace
        .prepare_transaction(&fixture(workspace_id, seed, TransactionMode::ValidateOnly))
        .expect("fixture validate-only");
    let committed = commit_checked(
        &mut workspace,
        &fixture(workspace_id, seed, TransactionMode::Commit),
    );
    let mut expected_prediction = predicted.receipt;
    expected_prediction.published = true;
    assert_eq!(committed, expected_prediction);

    let package = binding(&committed, 1);
    let module = binding(&committed, 2);
    let forty = binding(&committed, 7);
    let two = binding(&committed, 8);
    let boolean = binding(&committed, 9);
    let hole = binding(&committed, 10);
    let later = binding(&committed, 11);
    let mut completed = BTreeSet::new();
    while completed.len() < Action::ALL.len() {
        let invalid_refinement_done = [
            Action::InvalidRefinement,
            Action::InvalidType,
            Action::InvalidOrder,
            Action::InvalidOutput,
        ]
        .into_iter()
        .all(|action| completed.contains(&action));
        let applicable: Vec<_> = Action::ALL
            .into_iter()
            .filter(|action| !completed.contains(action))
            .filter(|action| match action {
                Action::Refine => invalid_refinement_done,
                Action::OperandEdit => completed.contains(&Action::Refine),
                _ => true,
            })
            .collect();
        let action = applicable[random.index(applicable.len())];
        trace.push(action);
        let revision = workspace.head_revision();
        match action {
            Action::Rename => {
                commit_checked(
                    &mut workspace,
                    &request(
                        Transaction {
                            workspace: workspace_id,
                            base_revision: revision,
                            idempotency_key: None,
                            mode: TransactionMode::Commit,
                            operations: vec![TransactionOp::RenameNode {
                                node: existing(module),
                                name: format!("renamed-{}", random.next() % 1000),
                            }],
                        },
                        &[],
                    ),
                );
            }
            Action::ScalarEdit => {
                commit_checked(
                    &mut workspace,
                    &request(
                        Transaction {
                            workspace: workspace_id,
                            base_revision: revision,
                            idempotency_key: None,
                            mode: TransactionMode::Commit,
                            operations: vec![TransactionOp::ReplaceOperation {
                                operation: existing(forty),
                                replacement: OperationDraft::ConstI64(
                                    100 + i64::try_from(random.next() % 100).expect("small"),
                                ),
                            }],
                        },
                        &[],
                    ),
                );
            }
            Action::Refine => {
                let from = workspace.head().expect("pre-refinement").clone();
                commit_checked(
                    &mut workspace,
                    &request(
                        Transaction {
                            workspace: workspace_id,
                            base_revision: revision,
                            idempotency_key: None,
                            mode: TransactionMode::Commit,
                            operations: vec![TransactionOp::RefineHole {
                                hole: existing(hole),
                                replacement: OperationDraft::AddI64 {
                                    lhs: existing_value(forty, 0),
                                    rhs: existing_value(two, 0),
                                },
                            }],
                        },
                        &[],
                    ),
                );
                assert!(
                    diff::between(&from, workspace.head().expect("refined"))
                        .changes
                        .iter()
                        .any(|change| matches!(
                            change.kind,
                            diff::ChangeKind::OperationRefined { .. }
                        ))
                );
            }
            Action::OperandEdit => {
                commit_checked(
                    &mut workspace,
                    &request(
                        Transaction {
                            workspace: workspace_id,
                            base_revision: revision,
                            idempotency_key: None,
                            mode: TransactionMode::Commit,
                            operations: vec![TransactionOp::ReplaceOperand {
                                operation: existing(hole),
                                index: 1,
                                value: existing_value(forty, 0),
                            }],
                        },
                        &[],
                    ),
                );
            }
            Action::CreateDelete => {
                let from = workspace.head().expect("pre-tombstone").clone();
                let receipt = commit_checked(
                    &mut workspace,
                    &request(
                        Transaction {
                            workspace: workspace_id,
                            base_revision: revision,
                            idempotency_key: None,
                            mode: TransactionMode::Commit,
                            operations: vec![
                                TransactionOp::CreatePackage {
                                    symbol: DraftSymbol::generated(30),
                                    name: format!("temporary-{seed}"),
                                },
                                TransactionOp::DeleteOwnedSubtree { root: local(30) },
                            ],
                        },
                        &[30],
                    ),
                );
                let tombstone = binding(&receipt, 30);
                assert!(
                    workspace
                        .head()
                        .expect("head")
                        .contains_tombstone(tombstone.serial())
                );
                assert!(
                    diff::between(&from, workspace.head().expect("tombstone"))
                        .changes
                        .iter()
                        .any(|change| change.node == tombstone
                            && matches!(change.kind, diff::ChangeKind::AllocatedAndTombstoned))
                );
            }
            Action::ValidateThenCommit => {
                let mut candidate = request(
                    Transaction {
                        workspace: workspace_id,
                        base_revision: revision,
                        idempotency_key: None,
                        mode: TransactionMode::ValidateOnly,
                        operations: vec![TransactionOp::CreatePackage {
                            symbol: DraftSymbol::generated(31),
                            name: format!("validated-{seed}"),
                        }],
                    },
                    &[31],
                );
                let predicted = workspace
                    .prepare_transaction(&candidate)
                    .expect("state-aware validate-only")
                    .receipt;
                candidate.transaction.mode = TransactionMode::Commit;
                let committed = commit_checked(&mut workspace, &candidate);
                let mut expected = predicted;
                expected.published = true;
                assert_eq!(committed, expected);
            }
            Action::StaleRevision => reject_checked(
                &workspace,
                &request(
                    Transaction {
                        workspace: workspace_id,
                        base_revision: Revision::INITIAL,
                        idempotency_key: None,
                        mode: TransactionMode::Commit,
                        operations: vec![TransactionOp::RenameNode {
                            node: existing(package),
                            name: "stale".to_owned(),
                        }],
                    },
                    &[],
                ),
                &format!("prediction-{seed}"),
                ErrorCode::RevisionConflict,
            ),
            Action::WrongWorkspace => reject_checked(
                &workspace,
                &request(
                    Transaction {
                        workspace: workspace_id,
                        base_revision: revision,
                        idempotency_key: None,
                        mode: TransactionMode::Commit,
                        operations: vec![TransactionOp::RenameNode {
                            node: existing(NodeId::new(other, package.serial()).expect("foreign")),
                            name: "foreign".to_owned(),
                        }],
                    },
                    &[],
                ),
                &format!("prediction-{seed}"),
                ErrorCode::WrongWorkspace,
            ),
            Action::DuplicateDraftSymbol => reject_checked(
                &workspace,
                &request(
                    Transaction {
                        workspace: workspace_id,
                        base_revision: revision,
                        idempotency_key: None,
                        mode: TransactionMode::Commit,
                        operations: vec![
                            TransactionOp::CreatePackage {
                                symbol: DraftSymbol::generated(40),
                                name: "duplicate-a".to_owned(),
                            },
                            TransactionOp::CreatePackage {
                                symbol: DraftSymbol::generated(40),
                                name: "duplicate-b".to_owned(),
                            },
                        ],
                    },
                    &[],
                ),
                &format!("prediction-{seed}"),
                ErrorCode::DuplicateDraftSymbol,
            ),
            Action::DuplicateName => reject_checked(
                &workspace,
                &request(
                    Transaction {
                        workspace: workspace_id,
                        base_revision: revision,
                        idempotency_key: None,
                        mode: TransactionMode::Commit,
                        operations: vec![TransactionOp::CreateFunction {
                            symbol: DraftSymbol::generated(41),
                            module: existing(module),
                            name: "main".to_owned(),
                            parameters: Vec::new(),
                            result: SemanticType::I64.into(),
                            body: None,
                        }],
                    },
                    &[41],
                ),
                &format!("prediction-{seed}"),
                ErrorCode::DuplicateName,
            ),
            Action::InvalidSelected => reject_checked(
                &workspace,
                &request(
                    Transaction {
                        workspace: workspace_id,
                        base_revision: revision,
                        idempotency_key: None,
                        mode: TransactionMode::Commit,
                        operations: vec![TransactionOp::CreatePackage {
                            symbol: DraftSymbol::generated(42),
                            name: "selected".to_owned(),
                        }],
                    },
                    &[43],
                ),
                &format!("prediction-{seed}"),
                ErrorCode::InvalidDraftSymbol,
            ),
            Action::StructuredScenario => generated_structured_scenario(seed),
            Action::InvalidRefinement => reject_checked(
                &workspace,
                &request(
                    Transaction {
                        workspace: workspace_id,
                        base_revision: revision,
                        idempotency_key: None,
                        mode: TransactionMode::Commit,
                        operations: vec![TransactionOp::RefineHole {
                            hole: existing(hole),
                            replacement: OperationDraft::Hole {
                                expected: SemanticType::I64.into(),
                            },
                        }],
                    },
                    &[],
                ),
                &format!("prediction-{seed}"),
                ErrorCode::InvalidOperand,
            ),
            Action::InvalidType => reject_checked(
                &workspace,
                &request(
                    Transaction {
                        workspace: workspace_id,
                        base_revision: revision,
                        idempotency_key: None,
                        mode: TransactionMode::Commit,
                        operations: vec![TransactionOp::RefineHole {
                            hole: existing(hole),
                            replacement: OperationDraft::AddI64 {
                                lhs: existing_value(forty, 0),
                                rhs: existing_value(boolean, 0),
                            },
                        }],
                    },
                    &[],
                ),
                &format!("prediction-{seed}"),
                ErrorCode::TypeMismatch,
            ),
            Action::InvalidOrder => reject_checked(
                &workspace,
                &request(
                    Transaction {
                        workspace: workspace_id,
                        base_revision: revision,
                        idempotency_key: None,
                        mode: TransactionMode::Commit,
                        operations: vec![TransactionOp::RefineHole {
                            hole: existing(hole),
                            replacement: OperationDraft::AddI64 {
                                lhs: existing_value(forty, 0),
                                rhs: existing_value(later, 0),
                            },
                        }],
                    },
                    &[],
                ),
                &format!("prediction-{seed}"),
                ErrorCode::InvalidOperand,
            ),
            Action::InvalidOutput => reject_checked(
                &workspace,
                &request(
                    Transaction {
                        workspace: workspace_id,
                        base_revision: revision,
                        idempotency_key: None,
                        mode: TransactionMode::Commit,
                        operations: vec![TransactionOp::RefineHole {
                            hole: existing(hole),
                            replacement: OperationDraft::AddI64 {
                                lhs: existing_value(forty, 1),
                                rhs: existing_value(two, 0),
                            },
                        }],
                    },
                    &[],
                ),
                &format!("prediction-{seed}"),
                ErrorCode::InvalidOperand,
            ),
        }
        completed.insert(action);
    }
}

fn directory_files(path: &std::path::Path) -> Vec<std::ffi::OsString> {
    let mut names: Vec<_> = fs::read_dir(path)
        .expect("directory")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    names.sort();
    names
}

fn durable_reject_checked(
    durable: &mut DurableWorkspace,
    directory: &std::path::Path,
    request: &ApplyTransactionRequest,
    expected: ErrorCode,
) {
    let before_head = fs::read(directory.join("HEAD")).expect("HEAD");
    let before_files = directory_files(&directory.join("revisions"));
    let before = durable.head().expect("head").clone();
    let before_tombstones: Vec<_> = before.tombstones().collect();
    let fingerprint = machine::transaction_fingerprint(request).expect("fingerprint");
    assert_eq!(
        durable
            .apply(request, fingerprint)
            .expect_err("invalid durable transaction")
            .code,
        expected
    );
    assert_eq!(fs::read(directory.join("HEAD")).expect("HEAD"), before_head);
    assert_eq!(directory_files(&directory.join("revisions")), before_files);
    let after = durable.head().expect("head");
    assert_eq!(after.revision(), before.revision());
    assert_eq!(after.hash(), before.hash());
    assert_eq!(after.next_serial(), before.next_serial());
    assert_eq!(after.tombstones().collect::<Vec<_>>(), before_tombstones);
}

fn generated_structured_scenario(seed: u64) {
    let mut bytes = [0x6d; 16];
    bytes[..8].copy_from_slice(&seed.to_le_bytes());
    let workspace_id = WorkspaceId::from_bytes(bytes);
    let mut workspace = Workspace::new(workspace_id).expect("structured workspace");
    let value = |symbol| ValueDraft::OperationResult {
        operation: local(symbol),
        output: 0,
    };
    let expression = |symbol, operation| ExpressionDraft {
        symbol: Some(DraftSymbol::generated(symbol)),
        operation,
    };
    let created = commit_checked(
        &mut workspace,
        &request(
            Transaction {
                workspace: workspace_id,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![
                    TransactionOp::CreatePackage {
                        symbol: DraftSymbol::generated(1),
                        name: "structured".into(),
                    },
                    TransactionOp::CreateModule {
                        symbol: DraftSymbol::generated(2),
                        package: local(1),
                        name: "root".into(),
                    },
                    TransactionOp::CreateFunction {
                        symbol: DraftSymbol::generated(10),
                        module: local(2),
                        name: "forward".into(),
                        parameters: Vec::new(),
                        result: SemanticType::I64.into(),
                        body: Some(FunctionBodyDraft {
                            operations: vec![expression(
                                11,
                                ExpressionKindDraft::Call {
                                    function: local(20),
                                    arguments: Vec::new(),
                                },
                            )],
                            return_value: value(11),
                        }),
                    },
                    TransactionOp::CreateFunction {
                        symbol: DraftSymbol::generated(20),
                        module: local(2),
                        name: "mutual".into(),
                        parameters: Vec::new(),
                        result: SemanticType::I64.into(),
                        body: Some(FunctionBodyDraft {
                            operations: vec![expression(
                                21,
                                ExpressionKindDraft::Call {
                                    function: local(10),
                                    arguments: Vec::new(),
                                },
                            )],
                            return_value: value(21),
                        }),
                    },
                    TransactionOp::CreateFunction {
                        symbol: DraftSymbol::generated(30),
                        module: local(2),
                        name: "nested".into(),
                        parameters: Vec::new(),
                        result: SemanticType::I64.into(),
                        body: Some(FunctionBodyDraft {
                            operations: vec![
                                expression(31, ExpressionKindDraft::ConstI64(0)),
                                expression(32, ExpressionKindDraft::ConstBool(true)),
                                expression(
                                    33,
                                    ExpressionKindDraft::If {
                                        condition: value(32),
                                        result: SemanticType::I64.into(),
                                        then_body: YieldingBodyDraft {
                                            operations: vec![expression(
                                                34,
                                                ExpressionKindDraft::ForI64 {
                                                    start: value(31),
                                                    end_exclusive: value(31),
                                                    step: 1,
                                                    initial: value(31),
                                                    carried: SemanticType::I64.into(),
                                                    index_symbol: DraftSymbol::generated(35),
                                                    carried_symbol: DraftSymbol::generated(36),
                                                    body: YieldingBodyDraft {
                                                        operations: vec![expression(
                                                            37,
                                                            ExpressionKindDraft::Hole {
                                                                expected: SemanticType::I64.into(),
                                                            },
                                                        )],
                                                        yield_value: value(37),
                                                    },
                                                },
                                            )],
                                            yield_value: value(34),
                                        },
                                        else_body: YieldingBodyDraft {
                                            operations: vec![expression(
                                                38,
                                                ExpressionKindDraft::ConstI64(0),
                                            )],
                                            yield_value: value(38),
                                        },
                                    },
                                ),
                            ],
                            return_value: value(33),
                        }),
                    },
                    TransactionOp::SetEntryFunction {
                        package: local(1),
                        function: local(30),
                    },
                ],
            },
            &[10, 20, 30, 35, 36, 37],
        ),
    );
    let hole = binding(&created, 37);
    let index = binding(&created, 35);
    let carried = binding(&created, 36);
    reject_checked(
        &workspace,
        &request(
            Transaction {
                workspace: workspace_id,
                base_revision: workspace.head_revision(),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::RefineHole {
                    hole: existing(hole),
                    replacement: OperationDraft::Hole {
                        expected: SemanticType::I64.into(),
                    },
                }],
            },
            &[],
        ),
        &format!("structured-prediction-{seed}"),
        ErrorCode::InvalidOperand,
    );
    let refinement_revision = workspace.head_revision();
    commit_checked(
        &mut workspace,
        &request(
            Transaction {
                workspace: workspace_id,
                base_revision: refinement_revision,
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::RefineHole {
                    hole: existing(hole),
                    replacement: OperationDraft::AddI64 {
                        lhs: ValueDraft::BlockArgument(existing(carried)),
                        rhs: ValueDraft::BlockArgument(existing(index)),
                    },
                }],
            },
            &[],
        ),
    );
    assert!(
        crate::query::workspace_blockers(workspace.head().expect("structured head")).is_empty()
    );
}

#[test]
fn durable_invalid_transaction_corpus_is_atomic_and_restart_stable() {
    let temporary = tempfile::tempdir().expect("state");
    persistence::ensure_state_directory(temporary.path()).expect("state directory");
    let id = WorkspaceId::from_bytes([0xd1; 16]);
    let mut durable = DurableWorkspace::create(temporary.path(), id).expect("durable workspace");
    let directory = persistence::workspace_directory(temporary.path(), id);
    let mut valid = fixture(id, 17, TransactionMode::Commit);
    valid.transaction.idempotency_key = Some(IdempotencyKey::from_bytes([0xa1; 16]));
    let mut prediction = valid.clone();
    prediction.transaction.idempotency_key = None;
    prediction.transaction.mode = TransactionMode::ValidateOnly;
    let prediction_fingerprint =
        machine::transaction_fingerprint(&prediction).expect("prediction fingerprint");
    let predicted = durable
        .apply(&prediction, prediction_fingerprint)
        .expect("fixture prediction");
    let fingerprint = machine::transaction_fingerprint(&valid).expect("fixture fingerprint");
    let committed = durable.apply(&valid, fingerprint).expect("fixture commit");
    assert_eq!(committed.returned_bindings, predicted.returned_bindings);
    assert_eq!(
        durable.apply(&valid, fingerprint).expect("exact replay"),
        committed
    );
    let hole = binding(&committed, 10);
    let forty = binding(&committed, 7);
    let two = binding(&committed, 8);
    let boolean = binding(&committed, 9);
    let later = binding(&committed, 11);
    let revision_one = artifact::encode(durable.snapshot(Revision::new(1)).expect("revision one"))
        .expect("revision one artifact");

    drop(durable);
    let mut durable = DurableWorkspace::open(temporary.path(), id).expect("meaningful restart");
    assert_eq!(
        durable
            .apply(&valid, fingerprint)
            .expect("replay after restart"),
        committed
    );
    assert_eq!(
        artifact::encode(durable.snapshot(Revision::new(1)).expect("revision one"))
            .expect("revision one artifact"),
        revision_one
    );

    let invalids = [
        (
            request(
                Transaction {
                    workspace: id,
                    base_revision: Revision::new(1),
                    idempotency_key: None,
                    mode: TransactionMode::Commit,
                    operations: vec![TransactionOp::RefineHole {
                        hole: existing(hole),
                        replacement: OperationDraft::AddI64 {
                            lhs: existing_value(forty, 0),
                            rhs: existing_value(boolean, 0),
                        },
                    }],
                },
                &[],
            ),
            ErrorCode::TypeMismatch,
        ),
        (
            request(
                Transaction {
                    workspace: id,
                    base_revision: Revision::new(1),
                    idempotency_key: None,
                    mode: TransactionMode::Commit,
                    operations: vec![TransactionOp::RefineHole {
                        hole: existing(hole),
                        replacement: OperationDraft::AddI64 {
                            lhs: existing_value(forty, 0),
                            rhs: existing_value(later, 0),
                        },
                    }],
                },
                &[],
            ),
            ErrorCode::InvalidOperand,
        ),
        (
            request(
                Transaction {
                    workspace: id,
                    base_revision: Revision::new(1),
                    idempotency_key: None,
                    mode: TransactionMode::Commit,
                    operations: vec![TransactionOp::RefineHole {
                        hole: existing(hole),
                        replacement: OperationDraft::AddI64 {
                            lhs: existing_value(forty, 1),
                            rhs: existing_value(two, 0),
                        },
                    }],
                },
                &[],
            ),
            ErrorCode::InvalidOperand,
        ),
        (
            request(
                Transaction {
                    workspace: id,
                    base_revision: Revision::new(1),
                    idempotency_key: None,
                    mode: TransactionMode::Commit,
                    operations: vec![TransactionOp::RefineHole {
                        hole: existing(hole),
                        replacement: OperationDraft::Hole {
                            expected: SemanticType::I64.into(),
                        },
                    }],
                },
                &[],
            ),
            ErrorCode::InvalidOperand,
        ),
    ];
    let mut random = Prng::new(0xd17);
    let mut remaining: Vec<_> = (0..invalids.len()).collect();
    while !remaining.is_empty() {
        let selected = random.index(remaining.len());
        let index = remaining.swap_remove(selected);
        let (invalid, expected) = &invalids[index];
        durable_reject_checked(&mut durable, &directory, invalid, *expected);
    }
    let mut conflict = valid.clone();
    conflict.transaction.operations[0] = TransactionOp::CreatePackage {
        symbol: DraftSymbol::generated(1),
        name: "conflict".to_owned(),
    };
    durable_reject_checked(
        &mut durable,
        &directory,
        &conflict,
        ErrorCode::IdempotencyConflict,
    );

    let refinement = request(
        Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: Some(IdempotencyKey::from_bytes([0xa2; 16])),
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: existing(hole),
                replacement: OperationDraft::AddI64 {
                    lhs: existing_value(forty, 0),
                    rhs: existing_value(two, 0),
                },
            }],
        },
        &[],
    );
    let refinement_fingerprint =
        machine::transaction_fingerprint(&refinement).expect("refinement fingerprint");
    let refined = durable
        .apply(&refinement, refinement_fingerprint)
        .expect("durable refinement");
    assert_eq!(
        durable
            .apply(&refinement, refinement_fingerprint)
            .expect("refinement replay"),
        refined
    );
    let revision_two = artifact::encode(durable.snapshot(Revision::new(2)).expect("revision two"))
        .expect("revision two artifact");
    let expected_diff = diff::between(
        durable.snapshot(Revision::new(1)).expect("revision one"),
        durable.snapshot(Revision::new(2)).expect("revision two"),
    );
    assert!(expected_diff.changes.iter().any(|change| {
        change.node == hole && matches!(change.kind, diff::ChangeKind::OperationRefined { .. })
    }));
    drop(durable);
    let mut reopened = DurableWorkspace::open(temporary.path(), id).expect("refined restart");
    assert_eq!(
        artifact::encode(reopened.snapshot(Revision::new(1)).expect("revision one"))
            .expect("revision one artifact"),
        revision_one
    );
    assert_eq!(
        artifact::encode(reopened.snapshot(Revision::new(2)).expect("revision two"))
            .expect("revision two artifact"),
        revision_two
    );
    assert_eq!(
        diff::between(
            reopened.snapshot(Revision::new(1)).expect("revision one"),
            reopened.snapshot(Revision::new(2)).expect("revision two"),
        ),
        expected_diff
    );
    assert_eq!(
        reopened
            .apply(&refinement, refinement_fingerprint)
            .expect("refinement replay after restart"),
        refined
    );
}

fn artifact_corpus() -> Vec<Vec<u8>> {
    let id = WorkspaceId::from_bytes([0xc1; 16]);
    let mut workspace = Workspace::new(id).expect("workspace");
    let empty = artifact::encode(workspace.head().expect("empty")).expect("empty artifact");
    let fixture_receipt = commit_checked(&mut workspace, &fixture(id, 1, TransactionMode::Commit));
    let incomplete = artifact::encode(workspace.head().expect("incomplete")).expect("artifact");
    let hole = binding(&fixture_receipt, 10);
    let forty = binding(&fixture_receipt, 7);
    let two = binding(&fixture_receipt, 8);
    commit_checked(
        &mut workspace,
        &request(
            Transaction {
                workspace: id,
                base_revision: Revision::new(1),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::RefineHole {
                    hole: existing(hole),
                    replacement: OperationDraft::AddI64 {
                        lhs: existing_value(forty, 0),
                        rhs: existing_value(two, 0),
                    },
                }],
            },
            &[],
        ),
    );
    let refined = artifact::encode(workspace.head().expect("refined")).expect("artifact");
    commit_checked(
        &mut workspace,
        &request(
            Transaction {
                workspace: id,
                base_revision: Revision::new(2),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![
                    TransactionOp::CreatePackage {
                        symbol: DraftSymbol::generated(100),
                        name: "discarded".to_owned(),
                    },
                    TransactionOp::DeleteOwnedSubtree { root: local(100) },
                ],
            },
            &[100],
        ),
    );
    let tombstoned = artifact::encode(workspace.head().expect("tombstoned")).expect("artifact");

    let block = match workspace.head().expect("head").node(hole).expect("hole") {
        crate::schema::Node::Operation { owner, .. } => *owner,
        _ => panic!("hole operation"),
    };
    let mut operations = Vec::new();
    for offset in 0..128_u32 {
        operations.push(TransactionOp::InsertExpression {
            block,
            before: Some(hole),
            expression: ExpressionDraft {
                symbol: Some(DraftSymbol::generated(1000 + offset)),
                operation: ExpressionKindDraft::ConstI64(i64::from(offset)),
            },
        });
    }
    commit_checked(
        &mut workspace,
        &request(
            Transaction {
                workspace: id,
                base_revision: Revision::new(3),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations,
            },
            &[1000],
        ),
    );
    let moderate = artifact::encode(workspace.head().expect("moderate")).expect("artifact");
    vec![empty, incomplete, refined, tombstoned, moderate]
}

fn request_corpus() -> Vec<Request> {
    let workspace = WorkspaceId::from_bytes([0xb1; 16]);
    let node = NodeId::new(workspace, 2).expect("node");
    let page = PageRequest {
        after: None,
        limit: 1,
    };
    let queries = vec![
        Query::WorkspaceSummary,
        Query::Node { node, expand: true },
        Query::Blockers { page },
        Query::OwnerChain { node, page },
        Query::Body { block: node, page },
        Query::IncomingUses {
            value: ValueRef::OperationResult {
                operation: node,
                output: 0,
            },
            page,
        },
        Query::DefinitionReferences { target: node, page },
        Query::Dependencies { node, page },
        Query::VisibleValues {
            purpose: VisibleCursorPurpose::VisibleValues,
            target: RepairTarget::Hole(node),
            include_incompatible: true,
            page,
        },
        Query::LegalConstructors {
            target: RepairTarget::Hole(node),
            include_incompatible: true,
            constructors: page,
            values: page,
        },
        Query::SemanticDiff {
            from: Revision::INITIAL,
            page,
        },
        Query::RepairContext {
            target: RepairTarget::Operand {
                operation: node,
                index: 0,
            },
            budget: ContextBudget {
                body_before: 1,
                body_after: 1,
                visible_values: 1,
                incoming_uses: 1,
                include_incompatible: true,
            },
        },
        Query::NominalType {
            declaration: node,
            page,
        },
    ];
    assert_eq!(
        queries.iter().map(Query::code).collect::<Vec<_>>(),
        crate::query::QueryCode::ALL
    );
    let transaction_operations = vec![
        TransactionOp::CreatePackage {
            symbol: DraftSymbol::generated(1),
            name: "package".to_owned(),
        },
        TransactionOp::CreateModule {
            symbol: DraftSymbol::generated(2),
            package: local(1),
            name: "module".to_owned(),
        },
        TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(3),
            module: local(2),
            name: "main".to_owned(),
            parameters: vec![FunctionParameterDraft {
                symbol: DraftSymbol::generated(4),
                name: "parameter".to_owned(),
                ty: SemanticType::I64.into(),
            }],
            result: SemanticType::I64.into(),
            body: None,
        },
        TransactionOp::DefineFunctionBody {
            function: node,
            body: FunctionBodyDraft {
                operations: vec![ExpressionDraft {
                    symbol: Some(DraftSymbol::generated(7)),
                    operation: ExpressionKindDraft::Hole {
                        expected: SemanticType::I64.into(),
                    },
                }],
                return_value: local_value(7),
            },
        },
        TransactionOp::ReplaceFunctionBody {
            function: node,
            body: FunctionBodyDraft {
                operations: Vec::new(),
                return_value: local_value(7),
            },
        },
        TransactionOp::InsertExpression {
            block: node,
            before: None,
            expression: ExpressionDraft {
                symbol: Some(DraftSymbol::generated(8)),
                operation: ExpressionKindDraft::ConstI64(1),
            },
        },
        TransactionOp::SetEntryFunction {
            package: local(1),
            function: local(3),
        },
        TransactionOp::RenameNode {
            node: local(2),
            name: "renamed".to_owned(),
        },
        TransactionOp::ReplaceOperation {
            operation: local(7),
            replacement: OperationDraft::ConstI64(1),
        },
        TransactionOp::ReplaceOperand {
            operation: local(7),
            index: 0,
            value: local_value(7),
        },
        TransactionOp::DeleteOwnedSubtree { root: local(4) },
        TransactionOp::RefineHole {
            hole: local(7),
            replacement: OperationDraft::Hole {
                expected: SemanticType::I64.into(),
            },
        },
        TransactionOp::CreateProductType {
            symbol: DraftSymbol::generated(9),
            module: local(2),
            name: "product".to_owned(),
            fields: Vec::new(),
        },
        TransactionOp::CreateSumType {
            symbol: DraftSymbol::generated(10),
            module: local(2),
            name: "sum".to_owned(),
            variants: vec![SumVariantDraft {
                symbol: DraftSymbol::generated(11),
                name: "variant".to_owned(),
                payload: None,
            }],
        },
        TransactionOp::CreateSequenceType {
            symbol: DraftSymbol::generated(12),
            module: local(2),
            name: "sequence".to_owned(),
            element: TypeDraft::I64,
        },
        TransactionOp::CreateBuildTarget {
            symbol: DraftSymbol::generated(13),
            name: "target".to_owned(),
            definition: crate::target::BuildTargetDefinition::Product(
                crate::target::ProductTargetDefinition { application: node },
            ),
        },
        TransactionOp::ReplaceBuildTarget {
            target: node,
            definition: crate::target::BuildTargetDefinition::Product(
                crate::target::ProductTargetDefinition { application: node },
            ),
        },
        TransactionOp::AddReleaseTargetExport {
            target: node,
            name: "legacy".to_owned(),
            item: node,
        },
        TransactionOp::SetReleaseTargetExport {
            target: node,
            name: "entry".to_owned(),
            item: node,
        },
        TransactionOp::SetApplicationQueryBoundary {
            target: node,
            query_entry: crate::target::TargetItem {
                release_target: node,
                item: node,
            },
            query: crate::target::TargetItem {
                release_target: node,
                item: node,
            },
        },
        TransactionOp::AddApplicationTargetTest {
            target: node,
            case: crate::target::TargetApplicationTestCase {
                name: "case".to_owned(),
                target: crate::target::TargetItem {
                    release_target: node,
                    item: node,
                },
                arguments: vec![crate::target::TargetValue::I64(1)],
                expected: crate::target::TargetTestExpectation::Value(
                    crate::target::TargetValue::I64(1),
                ),
                policy: crate::interpret::RunPolicy {
                    fuel: 1,
                    maximum_frames: 1,
                },
            },
        },
    ];
    assert_eq!(
        transaction_operations
            .iter()
            .map(TransactionOp::code)
            .collect::<Vec<_>>(),
        crate::transaction::TransactionOpCode::ALL
    );
    vec![
        Request::CreateWorkspace,
        Request::ApplyTransaction(request(
            Transaction {
                workspace,
                base_revision: Revision::new(1),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: transaction_operations,
            },
            &[1],
        )),
        Request::QueryBatch(QueryBatchRequest {
            workspace,
            revision: Revision::new(1),
            queries: queries
                .into_iter()
                .enumerate()
                .map(|(index, query)| QueryItem {
                    id: QueryId::new(u64::try_from(index).expect("query index") + 1),
                    query,
                })
                .collect(),
        }),
        Request::Run {
            workspace,
            revision: Revision::new(1),
            entry: node,
            arguments: vec![
                crate::RuntimeValue::Unit,
                crate::RuntimeValue::Bool(true),
                crate::RuntimeValue::I64(-9),
                crate::RuntimeValue::Product {
                    ty: node,
                    fields: vec![crate::RuntimeFieldValue {
                        field: node,
                        value: crate::RuntimeValue::Bool(false),
                    }],
                },
                crate::RuntimeValue::Sum {
                    ty: node,
                    variant: node,
                    payload: Some(Box::new(crate::RuntimeValue::I64(4))),
                },
            ],
            policy: crate::interpret::RunPolicy {
                fuel: 777,
                maximum_frames: 33,
            },
        },
        Request::Shutdown,
        Request::DescribeSchema(crate::machine::DescribeSchemaRequest::manifest()),
    ]
}

fn mutate_bytes(source: &[u8], seed: u64, case: u64) -> Vec<u8> {
    let mut random = Prng::new(seed ^ case.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    let mut bytes = source.to_vec();
    match case % 10 {
        0 => bytes.truncate(random.index(bytes.len().max(1)).min(bytes.len())),
        1 if !bytes.is_empty() => {
            let index = random.index(bytes.len());
            bytes[index] ^= 1 << (random.next() % 8);
        }
        2 => bytes.extend_from_slice(&[0xde, 0xad]),
        3 if bytes.len() >= 4 => bytes[..4].copy_from_slice(&u32::MAX.to_le_bytes()),
        4 if bytes.len() >= 8 => {
            let start = bytes.len() - 8;
            bytes[start..].fill(0xff);
        }
        5 if bytes.len() >= 2 => bytes[..2].fill(0xff),
        6 if bytes.len() >= 16 => bytes[8..16].fill(0),
        7 if !bytes.is_empty() => {
            let index = random.index(bytes.len());
            bytes[index] = 0xff;
        }
        8 if bytes.len() >= 8 => {
            let index = random.index(bytes.len() - 7);
            bytes[index..index + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        }
        _ => {
            let index = random.index(bytes.len().max(1)).min(bytes.len());
            bytes.insert(index, 0);
        }
    }
    bytes
}

fn mutate_json(source: &[u8], seed: u64, case: u64) -> Vec<u8> {
    let text = String::from_utf8(source.to_vec()).expect("valid corpus JSON");
    match case % 10 {
        0 => text.replacen("{", "{\"unknown\":0,", 1).into_bytes(),
        1 => text
            .replacen("\"version\":12", "\"version\":12,\"version\":12", 1)
            .into_bytes(),
        2 => text
            .replacen("\"request_id\":1", "\"request_id\":-1", 1)
            .into_bytes(),
        3 => text
            .replacen("\"kind\":", "\"kind\":\"unknown\",\"old_kind\":", 1)
            .into_bytes(),
        4 => text
            .replacen("\"request\":", "\"request\":null,\"old_request\":", 1)
            .into_bytes(),
        5 => format!("{text}{{}}").into_bytes(),
        6 => text.to_uppercase().into_bytes(),
        7 => text.replacen(":2", ":18446744073709551616", 1).into_bytes(),
        8 => format!("{}{}{}", "[".repeat(160), text, "]".repeat(160)).into_bytes(),
        _ => mutate_bytes(source, seed, case),
    }
}

fn exercise_artifact_mutation(corpus: &[Vec<u8>], seed: u64, case: u64) {
    let source = &corpus
        [usize::try_from((case / 10) % u64::try_from(corpus.len()).expect("len")).expect("index")];
    let mutated = mutate_bytes(source, seed, case);
    let first = artifact::decode(&mutated);
    let second = artifact::decode(&mutated);
    match (first, second) {
        (Ok(decoded), Ok(repeated)) => {
            assert_eq!(decoded, repeated);
            let canonical = artifact::encode(&decoded).expect("accepted artifact re-encodes");
            assert_eq!(
                canonical, mutated,
                "noncanonical artifact accepted: seed={seed} case={case}"
            );
        }
        (Err(first), Err(second)) => {
            assert_eq!(first.code, second.code);
            assert_eq!(first.message, second.message);
        }
        _ => panic!("artifact mutation classification changed: seed={seed} case={case}"),
    }
}

fn exercise_json_mutation(corpus: &[Vec<u8>], seed: u64, case: u64) {
    let source = &corpus
        [usize::try_from((case / 10) % u64::try_from(corpus.len()).expect("len")).expect("index")];
    let mutated = mutate_json(source, seed, case);
    let first = machine::decode_request(&mutated);
    let second = machine::decode_request(&mutated);
    match (first, second) {
        (Ok(decoded), Ok(repeated)) => {
            assert_eq!(decoded, repeated);
            let canonical = serde_json::to_vec(&decoded).expect("accepted JSON re-encodes");
            let canonical_decoded =
                machine::decode_request(&canonical).expect("canonical JSON decodes");
            assert_eq!(canonical_decoded, decoded);
        }
        (Err(first), Err(second)) => {
            assert_eq!(first.kind, second.kind);
            assert_eq!(first.message, second.message);
        }
        _ => panic!("JSON mutation classification changed: seed={seed} case={case}"),
    }
}

#[derive(Debug)]
struct NamedMutation {
    name: String,
    bytes: Vec<u8>,
}

fn push_mutation(
    mutations: &mut Vec<NamedMutation>,
    name: impl Into<String>,
    source: &[u8],
    bytes: Vec<u8>,
) {
    assert_ne!(bytes, source, "targeted mutation must change bytes");
    mutations.push(NamedMutation {
        name: name.into(),
        bytes,
    });
}

fn artifact_payload(bytes: &[u8]) -> Vec<u8> {
    let mut encoded = [0_u8; 8];
    encoded.copy_from_slice(&bytes[26..34]);
    let length = usize::try_from(u64::from_le_bytes(encoded)).expect("payload length");
    bytes[34..34 + length].to_vec()
}

fn rebuild_artifact(payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&artifact::MAGIC);
    bytes.extend_from_slice(&artifact::FORMAT_VERSION.0.to_le_bytes());
    bytes.extend_from_slice(&artifact::SCHEMA_ID.0);
    bytes.extend_from_slice(
        &u64::try_from(payload.len())
            .expect("payload length")
            .to_le_bytes(),
    );
    bytes.extend_from_slice(payload);
    bytes.extend_from_slice(&artifact::hash_payload(payload).as_bytes());
    bytes
}

fn targeted_artifact_mutations(corpus: &[Vec<u8>]) -> Vec<NamedMutation> {
    let empty = &corpus[0];
    let incomplete = &corpus[1];
    let mut mutations = Vec::new();
    for boundary in [0, 8, 10, 26, 34, empty.len() - 32, empty.len() - 1] {
        push_mutation(
            &mut mutations,
            format!("artifact-truncate-{boundary}"),
            empty,
            empty[..boundary].to_vec(),
        );
    }
    let mut length = empty.clone();
    length[26..34].copy_from_slice(&u64::MAX.to_le_bytes());
    push_mutation(&mut mutations, "artifact-length-inflation", empty, length);
    let mut count_payload = artifact_payload(empty);
    count_payload[48..56].copy_from_slice(&u64::MAX.to_le_bytes());
    push_mutation(
        &mut mutations,
        "artifact-node-count-inflation",
        empty,
        rebuild_artifact(&count_payload),
    );
    let mut trailing = empty.clone();
    trailing.push(0);
    push_mutation(&mut mutations, "artifact-trailing", empty, trailing);
    let mut hash = empty.clone();
    *hash.last_mut().expect("hash byte") ^= 1;
    push_mutation(&mut mutations, "artifact-hash", empty, hash);
    let mut workspace = empty.clone();
    workspace[34] ^= 1;
    push_mutation(&mut mutations, "artifact-workspace", empty, workspace);
    let mut invalid_utf8_payload = artifact_payload(incomplete);
    let name = invalid_utf8_payload
        .windows(b"package-1".len())
        .position(|window| window == b"package-1")
        .expect("fixture name");
    invalid_utf8_payload[name] = 0xff;
    push_mutation(
        &mut mutations,
        "artifact-invalid-utf8-name",
        incomplete,
        rebuild_artifact(&invalid_utf8_payload),
    );
    let mut duplicate_payload = artifact_payload(empty);
    duplicate_payload[48..56].copy_from_slice(&2_u64.to_le_bytes());
    duplicate_payload.extend_from_slice(&artifact_payload(empty)[56..]);
    push_mutation(
        &mut mutations,
        "artifact-duplicate-id",
        empty,
        rebuild_artifact(&duplicate_payload),
    );
    mutations
}

fn replace_json(source: &[u8], from: &str, to: &str) -> Vec<u8> {
    String::from_utf8(source.to_vec())
        .expect("JSON source")
        .replacen(from, to, 1)
        .into_bytes()
}

fn targeted_json_mutations(requests: &[Request]) -> Vec<NamedMutation> {
    let encoded: Vec<_> = requests
        .iter()
        .cloned()
        .map(|request| {
            serde_json::to_vec(&RequestEnvelope {
                version: machine::JSON_ENVELOPE_VERSION,
                request_id: RequestId::new(1),
                request,
            })
            .expect("JSON request")
        })
        .collect();
    let query = &encoded[2];
    let run = &encoded[3];
    let mut mutations = Vec::new();
    for (index, source) in encoded.iter().enumerate() {
        push_mutation(
            &mut mutations,
            format!("json-family-{index}-missing-required"),
            source,
            replace_json(source, "\"request_id\":1,", ""),
        );
    }
    push_mutation(
        &mut mutations,
        "json-unknown-field",
        query,
        replace_json(query, "{", "{\"unknown\":0,"),
    );
    push_mutation(
        &mut mutations,
        "json-duplicate-field",
        query,
        replace_json(query, "\"version\":12", "\"version\":12,\"version\":12"),
    );
    push_mutation(
        &mut mutations,
        "json-wrong-type",
        query,
        replace_json(query, "\"request_id\":1", "\"request_id\":\"one\""),
    );
    push_mutation(
        &mut mutations,
        "json-negative-unsigned",
        query,
        replace_json(query, "\"request_id\":1", "\"request_id\":-1"),
    );
    push_mutation(
        &mut mutations,
        "json-overflow-unsigned",
        query,
        replace_json(
            query,
            "\"request_id\":1",
            "\"request_id\":18446744073709551616",
        ),
    );
    let workspace = WorkspaceId::from_bytes([0xb1; 16]).to_string();
    push_mutation(
        &mut mutations,
        "json-invalid-hex",
        query,
        replace_json(query, &workspace, &format!("g{}", &workspace[1..])),
    );
    push_mutation(
        &mut mutations,
        "json-uppercase-noncanonical-hex",
        query,
        replace_json(query, &workspace, &workspace.to_uppercase()),
    );
    push_mutation(
        &mut mutations,
        "json-zero-node",
        run,
        replace_json(run, ":2\"", ":0\""),
    );
    let mut trailing = query.clone();
    trailing.extend_from_slice(b"{}");
    push_mutation(&mut mutations, "json-trailing", query, trailing);
    let deep = format!(
        "{}{}{}",
        "[".repeat(160),
        String::from_utf8(query.clone()).expect("query JSON"),
        "]".repeat(160)
    )
    .into_bytes();
    push_mutation(&mut mutations, "json-deep", query, deep);
    let oversized = vec![b' '; machine::MAX_JSON_INPUT_BYTES + 1];
    push_mutation(&mut mutations, "json-limit-plus-one", query, oversized);
    mutations
}

fn assert_targeted_artifact(mutation: &NamedMutation) {
    let first = artifact::decode(&mutation.bytes).expect_err(&mutation.name);
    let second = artifact::decode(&mutation.bytes).expect_err(&mutation.name);
    assert_eq!(first.code, second.code, "{}", mutation.name);
    assert_eq!(first.message, second.message, "{}", mutation.name);
}

fn assert_targeted_json(mutation: &NamedMutation) {
    let first = machine::decode_request(&mutation.bytes).expect_err(&mutation.name);
    let second = machine::decode_request(&mutation.bytes).expect_err(&mutation.name);
    assert_eq!(first.kind, second.kind, "{}", mutation.name);
    assert_eq!(first.message, second.message, "{}", mutation.name);
}

#[test]
fn named_targeted_boundary_mutations_are_stable() {
    let artifacts = artifact_corpus();
    for mutation in targeted_artifact_mutations(&artifacts) {
        assert_targeted_artifact(&mutation);
    }
    let requests = request_corpus();
    for mutation in targeted_json_mutations(&requests) {
        assert_targeted_json(&mutation);
    }
}

fn run_boundary_mutation(seed: u64, cases: u64) {
    let artifacts = artifact_corpus();
    let requests = request_corpus();
    let json: Vec<_> = requests
        .into_iter()
        .enumerate()
        .map(|(index, request)| {
            serde_json::to_vec(&RequestEnvelope {
                version: machine::JSON_ENVELOPE_VERSION,
                request_id: RequestId::new(u64::try_from(index).expect("request index") + 41),
                request,
            })
            .expect("JSON corpus")
        })
        .collect();
    for case in 0..cases {
        let boundary_case = case / 2;
        match case % 2 {
            0 => exercise_artifact_mutation(&artifacts, seed, boundary_case),
            _ => exercise_json_mutation(&json, seed, boundary_case),
        }
    }
}

#[test]
fn deterministic_boundary_mutation_normal_smoke() {
    run_boundary_mutation(0x1bad_c0de, 600);
}

/// Deterministic mutation smoke, not coverage-guided fuzzing.
///
/// Reproduce with:
/// `LKJSCRIPT_MUTATION_SEED=1 LKJSCRIPT_MUTATION_CASES=10000 cargo test --release boundary_mutation_smoke -- --ignored --nocapture --test-threads=1`
#[test]
#[ignore = "bounded manual deterministic mutation smoke"]
fn boundary_mutation_smoke() {
    let seed = std::env::var("LKJSCRIPT_MUTATION_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    let cases = std::env::var("LKJSCRIPT_MUTATION_CASES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(10_000);
    eprintln!("deterministic mutation smoke (not coverage-guided): seed={seed} cases={cases}");
    run_boundary_mutation(seed, cases);
}
