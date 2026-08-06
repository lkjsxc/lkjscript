use super::*;

#[test]
fn failure_cleanup_metadata_is_bounded_exact_and_independently_checked() {
    let mut chunk = Chunk::new();
    chunk.main.locals = 1;
    chunk.main.unique_places = 1;
    let size = chunk.add_const(crate::Constant::I64(1));
    chunk.main.emit_op_u16(Op::LoadConst, size.0);
    chunk.main.emit(Op::ByteVectorNew);
    chunk.main.emit_op_u8(Op::StoreUniqueLocal, 0);
    chunk.main.emit_op_u64_pair(Op::ByteVectorPlaceInit, 0, 0);
    let pop = u16::try_from(chunk.main.code.len()).expect("small cleanup fixture");
    chunk.main.emit(Op::Pop);
    let nop = u16::try_from(chunk.main.code.len()).expect("small cleanup fixture");
    chunk.main.emit(Op::Nop);
    let drop = u16::try_from(chunk.main.code.len()).expect("small cleanup fixture");
    chunk.main.emit_op_u64_pair(Op::ByteVectorDropPlace, 0, 0);
    let after_drop = u16::try_from(chunk.main.code.len()).expect("small cleanup fixture");
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u8(Op::ByteVectorPlaceEnd, 0);
    chunk.main.emit(Op::Pop);
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    chunk.main.failure_cleanups = vec![crate::FailureCleanupPlan {
        actions: vec![crate::FailureCleanupAction::DropUnique {
            local: 0,
            place: Some(0),
            kind: crate::UniqueValueKind::ByteVector,
        }],
    }];
    chunk.main.failure_cleanup_ranges = vec![
        crate::FailureCleanupRange {
            start: pop,
            end: nop,
            plan: Some(0),
            unentered_plan: None,
        },
        crate::FailureCleanupRange {
            start: nop,
            end: drop,
            plan: Some(0),
            unentered_plan: None,
        },
        crate::FailureCleanupRange {
            start: drop,
            end: after_drop,
            plan: Some(0),
            unentered_plan: None,
        },
    ];
    validate_chunk(chunk.clone(), &ValidationLimits::default())
        .expect("exact failure cleanup validates");
    let mut missing = chunk.clone();
    missing.main.failure_cleanup_ranges.remove(1);
    assert!(error(missing).contains("failure-cleanup"));
    let mut wrong_kind = chunk.clone();
    wrong_kind.main.failure_cleanups[0].actions[0] = crate::FailureCleanupAction::DropUnique {
        local: 0,
        place: Some(0),
        kind: crate::UniqueValueKind::Bytes,
    };
    let actual = error(wrong_kind);
    assert!(actual.contains("has wrong kind"), "{actual}");
    let mut invalid_unentered = chunk.clone();
    invalid_unentered.main.failure_cleanup_ranges[0].unentered_plan = Some(9);
    assert!(error(invalid_unentered).contains("malformed or overlapping"));
    let mut duplicate = chunk;
    let action = duplicate.main.failure_cleanups[0].actions[0];
    duplicate.main.failure_cleanups[0].actions.push(action);
    assert!(error(duplicate).contains("duplicates one local"));
}
