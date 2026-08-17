use super::*;
use crate::ids::DraftSymbol;
use crate::schema::{Node, OperationKind, SemanticType, TypeDraft, ValueDraft};
use crate::transaction::{
    ExpressionDraft, ExpressionKindDraft, FunctionBodyDraft, MatchArmDraft, NodeTarget,
    ProductFieldDraft, SumVariantDraft, Transaction, TransactionOp, TransactionResponseSpec,
    YieldingBodyDraft,
};

fn create_package(id: WorkspaceId) -> Transaction {
    Transaction {
        workspace: id,
        base_revision: Revision::INITIAL,
        idempotency_key: None,
        mode: TransactionMode::Commit,
        operations: vec![TransactionOp::CreatePackage {
            symbol: DraftSymbol::generated(1),
            name: "package".to_owned(),
        }],
    }
}

fn request(transaction: &Transaction) -> ApplyTransactionRequest {
    let mut return_symbols: Vec<DraftSymbol> = transaction
        .operations
        .iter()
        .filter_map(TransactionOp::created_symbol)
        .collect();
    return_symbols.sort();
    ApplyTransactionRequest {
        transaction: transaction.clone(),
        response: TransactionResponseSpec { return_symbols },
    }
}

#[test]
fn nominal_declarations_survive_format_six_restart_and_rederive_layout() {
    let temporary = tempfile::tempdir().expect("state");
    ensure_state_directory(temporary.path()).expect("state directory");
    let id = WorkspaceId::from_bytes([0x94; 16]);
    let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
    let transaction = Transaction {
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
                package: NodeTarget::Draft(DraftSymbol::generated(1)),
                name: "m".into(),
            },
            TransactionOp::CreateProductType {
                symbol: DraftSymbol::generated(3),
                module: NodeTarget::Draft(DraftSymbol::generated(2)),
                name: "Reading".into(),
                fields: vec![ProductFieldDraft {
                    symbol: DraftSymbol::generated(4),
                    name: "value".into(),
                    ty: TypeDraft::I64,
                }],
            },
        ],
    };
    workspace
        .apply(&request(&transaction), [0x94; 32])
        .expect("commit");
    drop(workspace);
    let reopened = DurableWorkspace::open(temporary.path(), id).expect("restart");
    let head = reopened.head().expect("head");
    assert_eq!(head.revision(), Revision::new(1));
    let declaration = head
        .nodes()
        .find_map(|(node, record)| {
            matches!(record, Node::ProductType { name, .. } if name == "Reading").then_some(node)
        })
        .expect("reading");
    let layouts = crate::type_layout::derive_layouts(head).expect("layouts");
    let crate::type_layout::DerivedLayout::Representable(layout) =
        layouts.get(&declaration).expect("layout")
    else {
        panic!("representable")
    };
    assert_eq!((layout.size, layout.align, layout.cells), (8, 8, 1));
}

#[test]
fn nominal_operation_and_match_graph_survives_format_six_restart_and_retained_query() {
    let temporary = tempfile::tempdir().expect("state");
    ensure_state_directory(temporary.path()).expect("state directory");
    let id = WorkspaceId::from_bytes([0x96; 16]);
    let local = |value| NodeTarget::Draft(DraftSymbol::generated(value));
    let result = |value| ValueDraft::OperationResult {
        operation: local(value),
        output: 0,
    };
    let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
    let transaction = Transaction {
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
                package: local(1),
                name: "m".into(),
            },
            TransactionOp::CreateSumType {
                symbol: DraftSymbol::generated(3),
                module: local(2),
                name: "Maybe".into(),
                variants: vec![
                    SumVariantDraft {
                        symbol: DraftSymbol::generated(4),
                        name: "none".into(),
                        payload: None,
                    },
                    SumVariantDraft {
                        symbol: DraftSymbol::generated(5),
                        name: "some".into(),
                        payload: Some(TypeDraft::I64),
                    },
                ],
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(6),
                module: local(2),
                name: "match_it".into(),
                parameters: Vec::new(),
                result: TypeDraft::I64,
                body: Some(FunctionBodyDraft {
                    operations: vec![
                        ExpressionDraft {
                            symbol: Some(DraftSymbol::generated(7)),
                            operation: ExpressionKindDraft::ConstI64(9),
                        },
                        ExpressionDraft {
                            symbol: Some(DraftSymbol::generated(8)),
                            operation: ExpressionKindDraft::ConstructVariant {
                                variant: local(5),
                                payload: Some(result(7)),
                            },
                        },
                        ExpressionDraft {
                            symbol: Some(DraftSymbol::generated(9)),
                            operation: ExpressionKindDraft::MatchSum {
                                scrutinee: result(8),
                                result: TypeDraft::I64,
                                arms: vec![
                                    MatchArmDraft {
                                        variant: local(5),
                                        payload_symbol: Some(DraftSymbol::generated(10)),
                                        body: YieldingBodyDraft {
                                            operations: Vec::new(),
                                            yield_value: ValueDraft::BlockArgument(local(10)),
                                        },
                                    },
                                    MatchArmDraft {
                                        variant: local(4),
                                        payload_symbol: None,
                                        body: YieldingBodyDraft {
                                            operations: vec![ExpressionDraft {
                                                symbol: Some(DraftSymbol::generated(11)),
                                                operation: ExpressionKindDraft::ConstI64(0),
                                            }],
                                            yield_value: result(11),
                                        },
                                    },
                                ],
                            },
                        },
                    ],
                    return_value: result(9),
                }),
            },
        ],
    };
    workspace
        .apply(&request(&transaction), [0x96; 32])
        .expect("commit nominal match");
    drop(workspace);

    let reopened = DurableWorkspace::open(temporary.path(), id).expect("artifact3 restart");
    let retained = reopened
        .snapshot(Revision::new(1))
        .expect("retained revision");
    let declaration = retained
        .nodes()
        .find_map(|(node, record)| {
            matches!(record, Node::SumType { name, .. } if name == "Maybe").then_some(node)
        })
        .expect("sum declaration");
    let queried = crate::query::execute(
        retained,
        &crate::query::Query::NominalType {
            declaration,
            page: crate::query::PageRequest {
                after: None,
                limit: 2,
            },
        },
        None,
    )
    .expect("retained nominal query");
    let crate::query::QueryResult::NominalType(queried) = queried else {
        panic!("nominal result")
    };
    assert_eq!(queried.name, "Maybe");
    assert_eq!(queried.members.items.len(), 2);
    let arms = retained
        .nodes()
        .find_map(|(_, node)| match node {
            Node::Operation {
                operation: OperationKind::MatchSum { arms, .. },
                ..
            } => Some(arms),
            _ => None,
        })
        .expect("retained match");
    assert_eq!(arms.len(), 2);
    let first_variant = match &queried.members.items[0] {
        crate::query::NominalMemberFact::SumVariant { variant, .. } => *variant,
        _ => panic!("sum member"),
    };
    assert_eq!(arms[0].variant, first_variant);
}

#[test]
fn state_directory_rejects_relative_and_symlinked_paths() {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::tempdir().expect("temporary directory");
    let real = temporary.path().join("real");
    fs::create_dir(&real).expect("real directory");
    let linked = temporary.path().join("linked");
    symlink(&real, &linked).expect("state symlink");
    assert_eq!(
        ensure_state_directory(&linked.join("state"))
            .expect_err("symlink component must reject")
            .code,
        ErrorCode::Io
    );
    assert_eq!(
        ensure_state_directory(Path::new("relative-state"))
            .expect_err("relative state must reject")
            .code,
        ErrorCode::Io
    );
}

#[test]
fn recognized_incomplete_workspace_staging_is_removed_on_startup() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    ensure_state_directory(temporary.path()).expect("state directory");
    let id = WorkspaceId::from_bytes([0x41; 16]);
    let staging = temporary
        .path()
        .join("workspaces")
        .join(format!(".creating-{id}-123-456"));
    fs::create_dir(&staging).expect("staging directory");
    fs::write(staging.join("partial"), b"partial").expect("partial file");
    assert!(
        list_workspace_ids(temporary.path())
            .expect("recover staging")
            .is_empty()
    );
    assert!(!staging.exists());
}

#[test]
fn noncanonical_workspace_and_revision_path_aliases_reject() {
    let workspace_state = tempfile::tempdir().expect("workspace alias state");
    ensure_state_directory(workspace_state.path()).expect("state directory");
    let workspace_id = WorkspaceId::from_bytes([0xab; 16]);
    DurableWorkspace::create(workspace_state.path(), workspace_id).expect("workspace");
    let canonical = workspace_directory(workspace_state.path(), workspace_id);
    let alias = canonical
        .parent()
        .expect("workspaces directory")
        .join(workspace_id.to_string().to_uppercase());
    fs::rename(&canonical, &alias).expect("rename to uppercase alias");
    assert_eq!(
        list_workspace_ids(workspace_state.path())
            .expect_err("uppercase workspace alias must reject")
            .code,
        ErrorCode::ArtifactCorrupt
    );

    let revision_state = tempfile::tempdir().expect("revision alias state");
    ensure_state_directory(revision_state.path()).expect("state directory");
    let revision_id = WorkspaceId::from_bytes([0x44; 16]);
    DurableWorkspace::create(revision_state.path(), revision_id).expect("workspace");
    let revisions = workspace_directory(revision_state.path(), revision_id).join("revisions");
    fs::rename(
        revision_path(&revisions, Revision::INITIAL),
        revisions.join("0.lkjscript"),
    )
    .expect("rename to decimal alias");
    assert_eq!(
        DurableWorkspace::open(revision_state.path(), revision_id)
            .err()
            .expect("decimal revision alias must reject")
            .code,
        ErrorCode::ArtifactCorrupt
    );
}

#[test]
fn only_strictly_named_temporary_files_are_recovered() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    ensure_state_directory(temporary.path()).expect("state directory");
    let id = WorkspaceId::from_bytes([0x42; 16]);
    DurableWorkspace::create(temporary.path(), id).expect("workspace");
    let directory = workspace_directory(temporary.path(), id);
    let recognized = directory.join(".tmp-123-456");
    fs::write(&recognized, b"partial").expect("recognized temporary");
    DurableWorkspace::open(temporary.path(), id).expect("recover recognized temporary");
    assert!(!recognized.exists());

    fs::write(directory.join(".tmp-not-owned"), b"unknown").expect("unknown file");
    assert_eq!(
        DurableWorkspace::open(temporary.path(), id)
            .err()
            .expect("unknown temporary must reject")
            .code,
        ErrorCode::ArtifactCorrupt
    );
}

#[test]
fn restart_rejects_history_that_clears_a_surviving_package_entry() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    ensure_state_directory(temporary.path()).expect("state directory");
    let id = WorkspaceId::from_bytes([0x45; 16]);
    let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
    let transaction = Transaction {
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
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: NodeTarget::Draft(DraftSymbol::generated(2)),
                name: "function".to_owned(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: None,
            },
            TransactionOp::SetEntryFunction {
                package: NodeTarget::Draft(DraftSymbol::generated(1)),
                function: NodeTarget::Draft(DraftSymbol::generated(3)),
            },
        ],
    };
    let accepted_request = request(&transaction);
    let fingerprint = machine::transaction_fingerprint(&accepted_request).expect("fingerprint");
    workspace
        .apply(&accepted_request, fingerprint)
        .expect("selected entry commit");
    let previous = workspace.head().expect("revision one").clone();
    let mut nodes = previous.nodes.clone();
    let package = nodes
        .iter_mut()
        .find_map(|(id, node)| match node {
            Node::Package { entry: Some(_), .. } => Some((*id, node)),
            _ => None,
        })
        .expect("selected package");
    let Node::Package { entry, .. } = package.1 else {
        panic!("package kind")
    };
    *entry = None;
    let forged = Snapshot::from_parts(
        id,
        Revision::new(2),
        previous.root,
        previous.next_serial,
        previous.tombstones.clone(),
        nodes,
    )
    .expect("individually valid cleared entry snapshot");
    let directory = workspace_directory(temporary.path(), id);
    fs::write(
        revision_path(&directory.join("revisions"), forged.revision()),
        artifact::encode(&forged).expect("forged artifact"),
    )
    .expect("write forged revision");
    fs::write(
        directory.join("HEAD"),
        encode_head(forged.revision(), forged.hash(), None).expect("forged HEAD"),
    )
    .expect("write forged HEAD");
    drop(workspace);
    assert_eq!(
        DurableWorkspace::open(temporary.path(), id)
            .err()
            .expect("restart must reject cleared entry history")
            .code,
        ErrorCode::ArtifactCorrupt
    );
}

#[test]
fn head_checksum_and_file_size_policy_reject_corrupt_durable_state() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    ensure_state_directory(temporary.path()).expect("state directory");

    let checksum_id = WorkspaceId::from_bytes([6; 16]);
    DurableWorkspace::create(temporary.path(), checksum_id).expect("workspace");
    let checksum_head = workspace_directory(temporary.path(), checksum_id).join("HEAD");
    let mut bytes = fs::read(&checksum_head).expect("head bytes");
    let last = bytes.last_mut().expect("head checksum byte");
    *last ^= 1;
    fs::write(&checksum_head, bytes).expect("corrupt head");
    assert_eq!(
        DurableWorkspace::open(temporary.path(), checksum_id)
            .err()
            .expect("corrupt HEAD must reject")
            .code,
        ErrorCode::ArtifactCorrupt
    );

    let oversized_head_id = WorkspaceId::from_bytes([0x70; 16]);
    DurableWorkspace::create(temporary.path(), oversized_head_id).expect("workspace");
    let oversized_head = workspace_directory(temporary.path(), oversized_head_id).join("HEAD");
    OpenOptions::new()
        .write(true)
        .open(&oversized_head)
        .expect("HEAD file")
        .set_len(u64::try_from(MAXIMUM_HEAD_BYTES).expect("HEAD policy") + 1)
        .expect("extend HEAD");
    assert_eq!(
        DurableWorkspace::open(temporary.path(), oversized_head_id)
            .err()
            .expect("oversized HEAD must reject before read")
            .code,
        ErrorCode::PolicyExceeded
    );

    let size_id = WorkspaceId::from_bytes([7; 16]);
    DurableWorkspace::create(temporary.path(), size_id).expect("workspace");
    let revision = revision_path(
        &workspace_directory(temporary.path(), size_id).join("revisions"),
        Revision::INITIAL,
    );
    let file = OpenOptions::new()
        .write(true)
        .open(revision)
        .expect("revision file");
    file.set_len(
        u64::try_from(artifact::DecodePolicy::default().maximum_artifact_bytes)
            .expect("artifact policy")
            + 1,
    )
    .expect("extend sparse revision");
    assert_eq!(
        DurableWorkspace::open(temporary.path(), size_id)
            .err()
            .expect("oversized revision must reject before read")
            .code,
        ErrorCode::PolicyExceeded
    );
}

#[test]
fn maximum_compact_receipt_keeps_head_below_explicit_policy() {
    let workspace = WorkspaceId::from_bytes([0x17; 16]);
    let returned_bindings = (0..crate::transaction::MAX_RETURNED_BINDINGS)
        .map(|index| {
            let prefix = format!("symbol_{index}_");
            let symbol = format!(
                "{prefix}{}",
                "x".repeat(crate::ids::MAX_DRAFT_SYMBOL_BYTES - prefix.len())
            );
            (
                DraftSymbol::new(&symbol),
                crate::ids::NodeId::new(workspace, u64::try_from(index).expect("serial") + 2)
                    .expect("node"),
            )
        })
        .collect();
    let record = IdempotencyRecord {
        key: IdempotencyKey::from_bytes([0x17; 16]),
        fingerprint: [0x23; 32],
        receipt: TransactionReceipt {
            workspace,
            base_revision: Revision::new(1),
            revision: Revision::new(2),
            hash: SnapshotHash::from_bytes([0x34; 32]),
            published: true,
            created_count: u64::MAX,
            returned_bindings,
            change_count: u64::MAX,
            change_digest: crate::ids::ChangeDigest::from_bytes([0x45; 32]),
            complete_before: false,
            complete_after: true,
            blocker_count_before: u64::MAX,
            blocker_count_after: 0,
        },
    };
    let bytes = encode_head(
        Revision::new(2),
        SnapshotHash::from_bytes([0x34; 32]),
        Some(&record),
    )
    .expect("maximum compact HEAD");
    assert!(bytes.len() < MAXIMUM_HEAD_BYTES);
    let (_, _, decoded) = decode_head(&bytes).expect("decode compact HEAD");
    assert_eq!(decoded.expect("idempotency").receipt, record.receipt);
}

#[test]
fn publication_rejects_live_head_tampering_before_replacement() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    ensure_state_directory(temporary.path()).expect("state directory");
    let id = WorkspaceId::from_bytes([0x19; 16]);
    let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
    let head = workspace.head().expect("head");
    let forged = encode_head(head.revision(), SnapshotHash::from_bytes([0x55; 32]), None)
        .expect("forged but decodable HEAD");
    let head_path = workspace_directory(temporary.path(), id).join("HEAD");
    fs::write(&head_path, &forged).expect("tamper live HEAD");
    let revision_before = fs::read_dir(workspace_directory(temporary.path(), id).join("revisions"))
        .expect("revisions")
        .count();
    let error = workspace
        .apply(&request(&create_package(id)), [0x22; 32])
        .expect_err("tampered live HEAD must reject");
    assert_eq!(error.code, ErrorCode::ArtifactCorrupt);
    assert_eq!(fs::read(&head_path).expect("HEAD remains tampered"), forged);
    assert_eq!(
        fs::read_dir(workspace_directory(temporary.path(), id).join("revisions"))
            .expect("revisions")
            .count(),
        revision_before
    );
    assert_eq!(
        workspace.head().expect("in-memory head").revision(),
        Revision::INITIAL
    );
}

#[test]
fn every_apply_path_verifies_live_head_before_replay_or_validation() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    ensure_state_directory(temporary.path()).expect("state directory");

    let replay_id = WorkspaceId::from_bytes([0x1a; 16]);
    let mut replay =
        DurableWorkspace::create(temporary.path(), replay_id).expect("replay workspace");
    let mut keyed = create_package(replay_id);
    keyed.idempotency_key = Some(IdempotencyKey::from_bytes([0x31; 16]));
    let keyed_request = request(&keyed);
    let fingerprint = machine::transaction_fingerprint(&keyed_request).expect("fingerprint");
    replay
        .apply(&keyed_request, fingerprint)
        .expect("keyed commit");
    let replay_head = workspace_directory(temporary.path(), replay_id).join("HEAD");
    let mut corrupt = fs::read(&replay_head).expect("HEAD");
    *corrupt.last_mut().expect("checksum") ^= 1;
    fs::write(&replay_head, corrupt).expect("corrupt replay HEAD");
    assert_eq!(
        replay
            .apply(&keyed_request, fingerprint)
            .expect_err("corrupt exact replay")
            .code,
        ErrorCode::ArtifactCorrupt
    );

    let conflict_id = WorkspaceId::from_bytes([0x1b; 16]);
    let mut conflict =
        DurableWorkspace::create(temporary.path(), conflict_id).expect("conflict workspace");
    let mut first = create_package(conflict_id);
    first.idempotency_key = Some(IdempotencyKey::from_bytes([0x32; 16]));
    let first_request = request(&first);
    let first_fingerprint = machine::transaction_fingerprint(&first_request).expect("fingerprint");
    conflict
        .apply(&first_request, first_fingerprint)
        .expect("keyed commit");
    let head = conflict.head().expect("head");
    let forged = encode_head(
        head.revision(),
        SnapshotHash::from_bytes([0x77; 32]),
        conflict.idempotency.as_ref(),
    )
    .expect("forged HEAD");
    fs::write(
        workspace_directory(temporary.path(), conflict_id).join("HEAD"),
        forged,
    )
    .expect("replace HEAD");
    let mut different = first_request.clone();
    different.transaction.operations[0] = TransactionOp::RenameNode {
        node: NodeTarget::Existing(head.root()),
        name: "different".into(),
    };
    let different_fingerprint = machine::transaction_fingerprint(&different).expect("fingerprint");
    assert_eq!(
        conflict
            .apply(&different, different_fingerprint)
            .expect_err("replaced conflict HEAD")
            .code,
        ErrorCode::ArtifactCorrupt
    );

    let validate_id = WorkspaceId::from_bytes([0x1c; 16]);
    let mut validate =
        DurableWorkspace::create(temporary.path(), validate_id).expect("validate workspace");
    fs::remove_file(workspace_directory(temporary.path(), validate_id).join("HEAD"))
        .expect("remove HEAD");
    let mut validate_transaction = create_package(validate_id);
    validate_transaction.mode = TransactionMode::ValidateOnly;
    assert_eq!(
        validate
            .apply(&request(&validate_transaction), [0x44; 32])
            .expect_err("missing validate HEAD")
            .code,
        ErrorCode::ArtifactCorrupt
    );

    let prepare_id = WorkspaceId::from_bytes([0x1d; 16]);
    let mut prepare =
        DurableWorkspace::create(temporary.path(), prepare_id).expect("prepare workspace");
    let prepare_head = workspace_directory(temporary.path(), prepare_id).join("HEAD");
    fs::write(&prepare_head, b"not a HEAD").expect("corrupt HEAD");
    assert_eq!(
        prepare
            .apply(&request(&create_package(prepare_id)), [0x55; 32])
            .expect_err("corrupt preparation HEAD")
            .code,
        ErrorCode::ArtifactCorrupt
    );
}

#[test]
fn head8_domain_separated_grammar_remains_fixed_and_deterministic() {
    let revision = Revision::new(7);
    let hash = SnapshotHash::from_bytes([0xa5; SnapshotHash::BYTE_LEN]);
    let first = encode_head(revision, hash, None).expect("HEAD8 encode");
    assert_eq!(
        first,
        encode_head(revision, hash, None).expect("deterministic HEAD8")
    );

    let mut expected_body = Vec::new();
    expected_body.extend_from_slice(b"LKJHEAD8");
    expected_body.extend_from_slice(&7_u64.to_le_bytes());
    expected_body.extend_from_slice(&[0xa5; SnapshotHash::BYTE_LEN]);
    expected_body.push(0);
    assert_eq!(&first[..expected_body.len()], expected_body.as_slice());
    assert_eq!(first.len(), expected_body.len() + SnapshotHash::BYTE_LEN);
    assert_eq!(
        &first[expected_body.len()..],
        head_checksum(&expected_body).as_slice()
    );
    let (decoded_revision, decoded_hash, decoded_record) =
        decode_head(&first).expect("HEAD8 decode");
    assert_eq!((decoded_revision, decoded_hash), (revision, hash));
    assert!(decoded_record.is_none());
}

#[test]
fn exact_commit_response_preflight_uses_real_id_and_fails_before_publication() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    ensure_state_directory(temporary.path()).expect("state directory");
    let id = WorkspaceId::from_bytes([0x67; 16]);
    let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
    let request = request(&create_package(id));
    let fingerprint = machine::transaction_fingerprint(&request).expect("fingerprint");
    let head_path = workspace_directory(temporary.path(), id).join("HEAD");
    let head_before = fs::read(&head_path).expect("HEAD before preflight");

    assert_eq!(
        workspace
            .apply_with_response(&request, fingerprint, crate::ids::RequestId::new(0))
            .expect_err("zero request ID must fail exact response preflight")
            .code,
        ErrorCode::PolicyExceeded
    );
    assert_eq!(
        fs::read(&head_path).expect("HEAD after preflight"),
        head_before
    );
    assert_eq!(
        workspace.head().expect("in-memory head").revision(),
        Revision::INITIAL
    );

    let (receipt, bytes) = workspace
        .apply_with_response(&request, fingerprint, crate::ids::RequestId::new(91))
        .expect("commit with exact response preflight");
    let envelope = machine::decode_response(&bytes).expect("preflighted response JSON");
    assert_eq!(envelope.request_id, crate::ids::RequestId::new(91));
    assert_eq!(
        envelope.response,
        crate::protocol::Response::TransactionReceipt(receipt)
    );
}

#[test]
fn head_version_seven_magic_is_rejected_without_compatibility_reader() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    ensure_state_directory(temporary.path()).expect("state directory");
    let id = WorkspaceId::from_bytes([0x18; 16]);
    DurableWorkspace::create(temporary.path(), id).expect("workspace");
    let head_path = workspace_directory(temporary.path(), id).join("HEAD");
    let mut bytes = fs::read(&head_path).expect("head bytes");
    bytes[..8].copy_from_slice(b"LKJHEAD7");
    let body_length = bytes.len() - SnapshotHash::BYTE_LEN;
    let checksum = head_checksum(&bytes[..body_length]);
    bytes[body_length..].copy_from_slice(&checksum);
    fs::write(&head_path, bytes).expect("old head magic");
    assert_eq!(
        DurableWorkspace::open(temporary.path(), id)
            .err()
            .expect("HEAD7 must reject")
            .code,
        ErrorCode::ArtifactCorrupt
    );
}

#[test]
fn persisted_idempotency_receipt_is_semantically_validated() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    ensure_state_directory(temporary.path()).expect("state directory");
    let id = WorkspaceId::from_bytes([9; 16]);
    let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
    let mut transaction = create_package(id);
    transaction.idempotency_key = Some(IdempotencyKey::from_bytes([0x91; 16]));
    let request = request(&transaction);
    let fingerprint = machine::transaction_fingerprint(&request).expect("fingerprint");
    let accepted = workspace
        .apply(&request, fingerprint)
        .expect("keyed transaction");
    let mut conflicting = request.clone();
    conflicting.transaction.base_revision = Revision::new(999);
    let conflicting_fingerprint =
        machine::transaction_fingerprint(&conflicting).expect("conflicting fingerprint");
    assert_eq!(
        workspace
            .apply(&conflicting, conflicting_fingerprint)
            .expect_err("matching key with a future base must conflict")
            .code,
        ErrorCode::IdempotencyConflict
    );
    assert_eq!(
        workspace
            .apply(&request, fingerprint)
            .expect("exact replay"),
        accepted
    );
    let valid_record = workspace.idempotency.clone().expect("idempotency record");
    let head = workspace.head().expect("head");
    let head_revision = head.revision();
    let head_hash = head.hash();
    let root = head.root();
    let head_path = workspace.directory.join("HEAD");

    let mut unpublished = valid_record.clone();
    unpublished.receipt.published = false;
    let forged = encode_head(head_revision, head_hash, Some(&unpublished)).expect("forged HEAD");
    fs::write(&head_path, forged).expect("write forged HEAD");
    assert_eq!(
        DurableWorkspace::open(temporary.path(), id)
            .err()
            .expect("unpublished idempotency result")
            .code,
        ErrorCode::ArtifactCorrupt
    );

    let mut missing = valid_record.clone();
    missing.receipt.created_count = 0;
    let forged = encode_head(head_revision, head_hash, Some(&missing)).expect("forged HEAD");
    fs::write(&head_path, forged).expect("write forged HEAD");
    assert_eq!(
        DurableWorkspace::open(temporary.path(), id)
            .err()
            .expect("wrong idempotency created count")
            .code,
        ErrorCode::ArtifactCorrupt
    );

    let mut wrong_digest = valid_record.clone();
    wrong_digest.receipt.change_digest = crate::ids::ChangeDigest::from_bytes([0xff; 32]);
    let forged = encode_head(head_revision, head_hash, Some(&wrong_digest)).expect("forged HEAD");
    fs::write(&head_path, forged).expect("write forged HEAD");
    assert_eq!(
        DurableWorkspace::open(temporary.path(), id)
            .err()
            .expect("wrong idempotency digest")
            .code,
        ErrorCode::ArtifactCorrupt
    );

    let mut prior = valid_record;
    prior.receipt.returned_bindings[0].1 = root;
    let forged = encode_head(head_revision, head_hash, Some(&prior)).expect("forged HEAD");
    fs::write(head_path, forged).expect("write forged HEAD");
    assert_eq!(
        DurableWorkspace::open(temporary.path(), id)
            .err()
            .expect("prior identity allocation")
            .code,
        ErrorCode::ArtifactCorrupt
    );
}

#[test]
fn validate_only_and_commit_share_persistence_policy_preflight() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    ensure_state_directory(temporary.path()).expect("state directory");
    let id = WorkspaceId::from_bytes([8; 16]);
    let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
    let directory = workspace_directory(temporary.path(), id);
    let head_path = directory.join("HEAD");
    let before_head = fs::read(&head_path).expect("initial head");
    let revision_files = || {
        let mut names: Vec<_> = fs::read_dir(directory.join("revisions"))
            .expect("revision directory")
            .map(|entry| entry.expect("revision entry").file_name())
            .collect();
        names.sort();
        names
    };
    let before_revisions = revision_files();
    let mut transaction = Transaction {
        workspace: id,
        base_revision: Revision::INITIAL,
        idempotency_key: None,
        mode: TransactionMode::ValidateOnly,
        operations: vec![TransactionOp::CreatePackage {
            symbol: DraftSymbol::generated(1),
            name: "x".repeat(artifact::DecodePolicy::default().maximum_name_bytes + 1),
        }],
    };
    for mode in [TransactionMode::ValidateOnly, TransactionMode::Commit] {
        transaction.mode = mode;
        let request = request(&transaction);
        let fingerprint = machine::transaction_fingerprint(&request).expect("fingerprint");
        assert_eq!(
            workspace
                .apply(&request, fingerprint)
                .expect_err("unreopenable artifact must reject in both modes")
                .code,
            ErrorCode::PolicyExceeded
        );
        assert_eq!(fs::read(&head_path).expect("unchanged head"), before_head);
        assert_eq!(revision_files(), before_revisions);
        assert_eq!(
            workspace.head().expect("head").revision(),
            Revision::INITIAL
        );
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
    }
}

#[test]
fn keyed_head8_publication_faults_preserve_prior_replay_and_allocator() {
    let temporary = tempfile::tempdir().expect("temporary state directory");
    ensure_state_directory(temporary.path()).expect("state directory");
    let id = WorkspaceId::from_bytes([5; 16]);
    for fault in [
        PublicationStep::BeforeRevisionWrite,
        PublicationStep::AfterRevisionSync,
        PublicationStep::AfterRevisionRename,
        PublicationStep::AfterHeadSync,
        PublicationStep::AfterHeadRename,
    ] {
        let path = workspace_directory(temporary.path(), id);
        if path.exists() {
            fs::remove_dir_all(&path).expect("remove prior test workspace");
        }
        let mut workspace =
            DurableWorkspace::create(temporary.path(), id).expect("durable workspace creation");
        let mut prior_transaction = create_package(id);
        prior_transaction.idempotency_key = Some(IdempotencyKey::from_bytes([0x51; 16]));
        let prior_request = request(&prior_transaction);
        let prior_fingerprint =
            machine::transaction_fingerprint(&prior_request).expect("prior fingerprint");
        let prior_receipt = workspace
            .apply(&prior_request, prior_fingerprint)
            .expect("prior keyed commit");
        let package = prior_receipt.returned_bindings[0].1;
        let before = fs::read(path.join("HEAD")).expect("read prior keyed HEAD");

        let candidate = ApplyTransactionRequest {
            transaction: Transaction {
                workspace: id,
                base_revision: Revision::new(1),
                idempotency_key: Some(IdempotencyKey::from_bytes([0x52; 16])),
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: crate::transaction::NodeTarget::Existing(package),
                    name: "module".to_owned(),
                }],
            },
            response: TransactionResponseSpec {
                return_symbols: vec![DraftSymbol::generated(2)],
            },
        };
        let candidate_fingerprint =
            machine::transaction_fingerprint(&candidate).expect("candidate fingerprint");
        let error = workspace
            .apply_with_fault(&candidate, candidate_fingerprint, fault)
            .expect_err("fault must reject publication");
        assert_eq!(error.code, ErrorCode::Io);
        assert_eq!(workspace.head().expect("head").revision(), Revision::new(1));
        assert_eq!(workspace.head().expect("head").next_serial(), 3);
        assert_eq!(
            workspace
                .idempotency
                .as_ref()
                .expect("prior idempotency")
                .receipt,
            prior_receipt
        );
        assert_eq!(
            fs::read(path.join("HEAD")).expect("read head after fault"),
            before
        );

        let mut reopened =
            DurableWorkspace::open(temporary.path(), id).expect("prior HEAD must reopen");
        assert_eq!(reopened.head().expect("head").next_serial(), 3);
        assert_eq!(
            reopened
                .idempotency
                .as_ref()
                .expect("reopened idempotency")
                .receipt,
            prior_receipt
        );
        assert_eq!(
            reopened
                .apply(&prior_request, prior_fingerprint)
                .expect("prior exact replay after fault"),
            prior_receipt
        );
    }
    let _ = SemanticType::Unit;
}
