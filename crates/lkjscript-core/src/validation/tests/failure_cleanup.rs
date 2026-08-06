use super::*;

fn cleanup_id(raw: u64) -> crate::FailureCleanupId {
    crate::FailureCleanupId::new(raw)
}

#[test]
fn failure_cleanup_metadata_is_shared_exact_and_independently_checked() {
    let mut chunk = Chunk::new();
    chunk.main.locals = 1;
    chunk.main.unique_places = 1;
    let size = chunk.add_const(crate::Constant::I64(1));
    chunk.main.emit_op_u16(Op::LoadConst, size.0);
    chunk.main.emit(Op::ByteVectorNew);
    chunk.main.emit_op_u8(Op::StoreUniqueLocal, 0);
    chunk.main.emit_op_u64_pair(Op::ByteVectorPlaceInit, 0, 0);
    let pop = u64::try_from(chunk.main.code.len()).expect("small cleanup fixture");
    chunk.main.emit(Op::Pop);
    let nop = u64::try_from(chunk.main.code.len()).expect("small cleanup fixture");
    chunk.main.emit(Op::Nop);
    let drop = u64::try_from(chunk.main.code.len()).expect("small cleanup fixture");
    chunk.main.emit_op_u64_pair(Op::ByteVectorDropPlace, 0, 0);
    let after_drop = u64::try_from(chunk.main.code.len()).expect("small cleanup fixture");
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u8(Op::ByteVectorPlaceEnd, 0);
    chunk.main.emit(Op::Pop);
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    let action = crate::FailureCleanupAction::DropUnique {
        local: 0,
        place: Some(0),
        kind: crate::UniqueValueKind::ByteVector,
    };
    chunk.main.failure_cleanups = vec![crate::FailureCleanupNode { action, next: None }];
    chunk.main.failure_cleanup_ranges = vec![
        crate::FailureCleanupRange {
            start: pop,
            end: nop,
            plan: Some(crate::FailureCleanupRoots::single(cleanup_id(0))),
            unentered_plan: None,
        },
        crate::FailureCleanupRange {
            start: nop,
            end: drop,
            plan: Some(crate::FailureCleanupRoots::single(cleanup_id(0))),
            unentered_plan: None,
        },
        crate::FailureCleanupRange {
            start: drop,
            end: after_drop,
            plan: Some(crate::FailureCleanupRoots::single(cleanup_id(0))),
            unentered_plan: None,
        },
    ];
    validate_chunk(chunk.clone(), &ValidationLimits::default())
        .expect("exact shared failure cleanup validates");

    let mut missing = chunk.clone();
    missing.main.failure_cleanup_ranges.remove(1);
    assert!(error(missing).contains("failure-cleanup"));

    let mut wrong_kind = chunk.clone();
    wrong_kind.main.failure_cleanups[0].action = crate::FailureCleanupAction::DropUnique {
        local: 0,
        place: Some(0),
        kind: crate::UniqueValueKind::Bytes,
    };
    let actual = error(wrong_kind);
    assert!(actual.contains("has wrong kind"), "{actual}");

    let mut invalid_unentered = chunk.clone();
    invalid_unentered.main.failure_cleanup_ranges[0].unentered_plan = Some(cleanup_id(9));
    assert!(error(invalid_unentered).contains("malformed or overlapping"));

    let mut invalid_root = chunk.clone();
    invalid_root.main.failure_cleanup_ranges[0].plan =
        Some(crate::FailureCleanupRoots::single(cleanup_id(u64::MAX)));
    assert!(error(invalid_root).contains("malformed or overlapping"));

    let mut duplicate_node = chunk.clone();
    duplicate_node
        .main
        .failure_cleanups
        .push(duplicate_node.main.failure_cleanups[0]);
    assert!(error(duplicate_node).contains("interned uniquely"));

    let mut duplicate_local = chunk.clone();
    duplicate_local
        .main
        .failure_cleanups
        .push(crate::FailureCleanupNode {
            action,
            next: Some(cleanup_id(0)),
        });
    duplicate_local.main.failure_cleanup_ranges[0].plan =
        Some(crate::FailureCleanupRoots::single(cleanup_id(1)));
    assert!(error(duplicate_local).contains("duplicates one local"));
}

#[test]
fn malformed_cleanup_links_fail_before_runtime_indexing() {
    let mut base = unit_chunk();
    let action = crate::FailureCleanupAction::DropUnique {
        local: 0,
        place: None,
        kind: crate::UniqueValueKind::ByteVector,
    };
    base.main.locals = 1;
    base.main.failure_cleanups = vec![crate::FailureCleanupNode { action, next: None }];

    let mut self_link = base.clone();
    self_link.main.failure_cleanups[0].next = Some(cleanup_id(0));
    assert!(error(self_link).contains("prior nodes"));

    let mut forward = base.clone();
    forward.main.failure_cleanups[0].next = Some(cleanup_id(1));
    forward
        .main
        .failure_cleanups
        .push(crate::FailureCleanupNode {
            action: crate::FailureCleanupAction::DropUnique {
                local: 0,
                place: None,
                kind: crate::UniqueValueKind::Bytes,
            },
            next: None,
        });
    assert!(error(forward).contains("prior nodes"));

    let mut out_of_range = base;
    out_of_range.main.failure_cleanups[0].next = Some(cleanup_id(u64::MAX));
    assert!(error(out_of_range).contains("prior nodes"));
}

#[test]
fn shared_tail_preserves_action_order() {
    let tail = crate::FailureCleanupAction::DropUnique {
        local: 0,
        place: None,
        kind: crate::UniqueValueKind::ByteVector,
    };
    let first = crate::FailureCleanupAction::DropUnique {
        local: 1,
        place: None,
        kind: crate::UniqueValueKind::Bytes,
    };
    let nodes = [
        crate::FailureCleanupNode {
            action: tail,
            next: None,
        },
        crate::FailureCleanupNode {
            action: first,
            next: Some(cleanup_id(0)),
        },
    ];
    let mut actual = Vec::new();
    let mut root = Some(cleanup_id(1));
    while let Some(id) = root {
        let node = nodes[id.index().expect("host cleanup ID")];
        actual.push(node.action);
        root = node.next;
    }
    assert_eq!(actual, [first, tail]);
}
