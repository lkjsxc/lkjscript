use super::fixtures::*;
use crate::*;

#[test]
fn verifier_rejects_malformed_move_borrow_and_loan_facts() {
    let mut valid = one_block_program();
    valid.functions.push(Function {
        id: FunctionId::new(1),
        name: "owned-id".into(),
        signature: Signature::monomorphic(vec![SsaType::ByteVector], SsaType::ByteVector),
        places: vec![crate::PlaceMetadata {
            id: PlaceId::new(0),
            binding: crate::BindingId::new(0),
            ty: SsaType::ByteVector,
            drop_glue: Some(DropGlueIdentity::ByteVector),
        }],
        failure_cleanups: vec![
            FailureCleanupPlan {
                id: FailureCleanupId::new(0),
                actions: vec![FailureCleanupAction::DropOwner {
                    place: Some(PlaceId::new(0)),
                    value: ValueId::new(0),
                    glue: DropGlueIdentity::ByteVector,
                }],
            },
            FailureCleanupPlan {
                id: FailureCleanupId::new(1),
                actions: vec![FailureCleanupAction::DropOwner {
                    place: None,
                    value: ValueId::new(1),
                    glue: DropGlueIdentity::ByteVector,
                }],
            },
        ],
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: vec![BlockParameter {
                id: ValueId::new(0),
                ty: SsaType::ByteVector,
                owner_place: Some(PlaceId::new(0)),
                origin: Origin::SYNTHETIC,
            }],
            instructions: vec![Instruction {
                id: ValueId::new(1),
                ty: SsaType::ByteVector,
                kind: InstructionKind::Move {
                    place: PlaceId::new(0),
                    value: ValueId::new(0),
                },
                metadata: metadata_cleanup(EffectSet::PURE, 0),
            }],
            terminator: Terminator::Return(ValueId::new(1)),
            metadata: block_metadata_cleanup(1),
        }],
        origin: Origin::SYNTHETIC,
    });
    assert!(verify(valid.clone()).is_ok());
    let mut duplicate_place_end = valid.clone();
    duplicate_place_end.functions[1].blocks[0].instructions = vec![
        Instruction {
            id: ValueId::new(1),
            ty: SsaType::Unit,
            kind: InstructionKind::PlaceEnd {
                place: PlaceId::new(0),
            },
            metadata: metadata(EffectSet::PURE),
        },
        Instruction {
            id: ValueId::new(2),
            ty: SsaType::Unit,
            kind: InstructionKind::PlaceEnd {
                place: PlaceId::new(0),
            },
            metadata: metadata(EffectSet::PURE),
        },
        Instruction {
            id: ValueId::new(3),
            ty: byte_vector_type(),
            kind: InstructionKind::Move {
                place: PlaceId::new(0),
                value: ValueId::new(0),
            },
            metadata: metadata(EffectSet::PURE),
        },
    ];
    duplicate_place_end.functions[1]
        .failure_cleanups
        .truncate(1);
    duplicate_place_end.functions[1].blocks[0].instructions[0]
        .metadata
        .failure_cleanup = Some(FailureCleanupId::new(0));
    duplicate_place_end.functions[1].blocks[0]
        .metadata
        .failure_cleanup = None;
    duplicate_place_end.functions[1].blocks[0].terminator = Terminator::Return(ValueId::new(3));
    let end_error = verify(duplicate_place_end).expect_err("duplicate PlaceEnd must fail");
    assert!(
        end_error.to_string().contains("cannot erase"),
        "{end_error}"
    );

    let mut copied = valid.clone();
    copied.functions[1].blocks[0].instructions[0].kind = InstructionKind::Copy(ValueId::new(0));
    assert!(verify(copied).is_err());
    let mut wrong_move = valid.clone();
    wrong_move.functions[1].blocks[0].instructions[0].ty = SsaType::I64;
    assert!(verify(wrong_move).is_err());

    let mut unknown_place = valid.clone();
    let InstructionKind::Move { place, .. } =
        &mut unknown_place.functions[1].blocks[0].instructions[0].kind
    else {
        panic!("expected move fact");
    };
    *place = PlaceId::new(9);
    assert!(verify(unknown_place).is_err());
    let mut duplicate_place = valid.clone();
    let repeated_place = duplicate_place.functions[1].places[0].clone();
    duplicate_place.functions[1].places.push(repeated_place);
    assert!(verify(duplicate_place).is_err());

    let mut duplicate_loan = valid;
    duplicate_loan.functions[1].blocks[0].instructions = vec![
        Instruction {
            id: ValueId::new(1),
            ty: SsaType::ByteSlice,
            kind: InstructionKind::Borrow {
                place: PlaceId::new(0),
                loan: LoanId::new(0),
                kind: BorrowKind::Shared,
                value: ValueId::new(0),
            },
            metadata: metadata(EffectSet::PURE),
        },
        Instruction {
            id: ValueId::new(2),
            ty: SsaType::ByteSlice,
            kind: InstructionKind::Borrow {
                place: PlaceId::new(0),
                loan: LoanId::new(0),
                kind: BorrowKind::Shared,
                value: ValueId::new(0),
            },
            metadata: metadata(EffectSet::PURE),
        },
    ];
    duplicate_loan.functions[1].blocks[0].terminator = Terminator::Trap {
        value: ValueId::new(0),
    };
    assert!(verify(duplicate_loan).is_err());

    assert_cross_function_duplicate_loans();
}
