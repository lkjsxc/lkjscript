use super::fixtures::*;
use crate::*;

#[test]
fn verifier_rejects_malformed_move_borrow_and_loan_facts() {
    let mut valid = one_block_program();
    valid.functions.push(Function {
        id: FunctionId::new(1),
        name: "owned-id".into(),
        signature: Signature::monomorphic(
            vec![SsaType::Owned(Box::new(SsaType::Buf))],
            SsaType::Owned(Box::new(SsaType::Buf)),
        ),
        places: vec![crate::PlaceMetadata {
            id: PlaceId::new(0),
            binding: crate::BindingId::new(0),
            ty: SsaType::Owned(Box::new(SsaType::Buf)),
        }],
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: vec![BlockParameter {
                id: ValueId::new(0),
                ty: SsaType::Owned(Box::new(SsaType::Buf)),
                owner_place: Some(PlaceId::new(0)),
                origin: Origin::SYNTHETIC,
            }],
            instructions: vec![Instruction {
                id: ValueId::new(1),
                ty: SsaType::Owned(Box::new(SsaType::Buf)),
                kind: InstructionKind::Move {
                    place: PlaceId::new(0),
                    value: ValueId::new(0),
                },
                metadata: metadata(EffectSet::PURE),
            }],
            terminator: Terminator::Return(ValueId::new(1)),
            metadata: block_metadata(),
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
            ty: owned_buf_type(),
            kind: InstructionKind::Move {
                place: PlaceId::new(0),
                value: ValueId::new(0),
            },
            metadata: metadata(EffectSet::PURE),
        },
    ];
    duplicate_place_end.functions[1].blocks[0].terminator = Terminator::Return(ValueId::new(3));
    let end_error = verify(duplicate_place_end).expect_err("duplicate PlaceEnd must fail");
    assert!(end_error.to_string().contains("not active"), "{end_error}");

    let mut copied = valid.clone();
    copied.functions[1].blocks[0].instructions[0].kind = InstructionKind::Copy(ValueId::new(0));
    assert!(verify(copied).is_err());

    let mut wrong_move = valid.clone();
    wrong_move.functions[1].blocks[0].instructions[0].ty = SsaType::Buf;
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
            ty: SsaType::Ref(Box::new(SsaType::Buf)),
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
            ty: SsaType::Ref(Box::new(SsaType::Buf)),
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

    let borrowed_function = |id: u32, name: &str| Function {
        id: FunctionId::new(id),
        name: name.into(),
        signature: Signature::monomorphic(vec![owned_buf_type()], SsaType::I64),
        places: vec![owned_place(0, 0)],
        effects: EffectSet::READS_MEMORY,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: vec![BlockParameter {
                id: ValueId::new(0),
                ty: owned_buf_type(),
                owner_place: Some(PlaceId::new(0)),
                origin: Origin::SYNTHETIC,
            }],
            instructions: vec![
                Instruction {
                    id: ValueId::new(1),
                    ty: SsaType::Ref(Box::new(SsaType::Buf)),
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
                    ty: SsaType::I64,
                    kind: InstructionKind::Runtime {
                        operation: RuntimeOp::OwnedBufLen,
                        arguments: vec![ValueId::new(1)],
                        signature: Signature::monomorphic(
                            vec![SsaType::Ref(Box::new(SsaType::Buf))],
                            SsaType::I64,
                        ),
                    },
                    metadata: metadata(EffectSet::READS_MEMORY),
                },
            ],
            terminator: Terminator::Return(ValueId::new(2)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    };
    let mut cross_function_duplicate = one_block_program();
    cross_function_duplicate.functions.extend([
        borrowed_function(1, "borrow-one"),
        borrowed_function(2, "borrow-two"),
    ]);
    let error = verify(cross_function_duplicate)
        .expect_err("LoanIds must be unique across the whole public Program");
    assert!(
        error.to_string().contains("anywhere in the program"),
        "{error}"
    );
}
