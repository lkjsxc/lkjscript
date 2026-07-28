use super::*;
use crate::*;

pub(crate) fn assert_cross_function_duplicate_loans() {
    let borrowed_function = |id: u32, name: &str| {
        let mut function = Function {
            id: FunctionId::new(id),
            name: name.into(),
            signature: Signature::monomorphic(vec![owned_buf_type()], SsaType::I64),
            places: vec![owned_place(0, 0)],
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
                    actions: vec![
                        FailureCleanupAction::EndBorrow {
                            place: PlaceId::new(0),
                            loan: LoanId::new(0),
                            kind: BorrowKind::Shared,
                            value: ValueId::new(1),
                        },
                        FailureCleanupAction::DropOwner {
                            place: Some(PlaceId::new(0)),
                            value: ValueId::new(0),
                            glue: DropGlueIdentity::ByteVector,
                        },
                    ],
                },
            ],
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
                        ty: SsaType::ByteSlice,
                        kind: InstructionKind::Borrow {
                            place: PlaceId::new(0),
                            loan: LoanId::new(0),
                            kind: BorrowKind::Shared,
                            value: ValueId::new(0),
                        },
                        metadata: metadata_cleanup(EffectSet::PURE, 0),
                    },
                    Instruction {
                        id: ValueId::new(2),
                        ty: SsaType::I64,
                        kind: InstructionKind::Runtime {
                            operation: RuntimeOp::OwnedBufLen,
                            arguments: vec![ValueId::new(1)],
                            signature: Signature::monomorphic(
                                vec![SsaType::ByteSlice],
                                SsaType::I64,
                            ),
                        },
                        metadata: metadata_cleanup(EffectSet::READS_MEMORY, 1),
                    },
                    Instruction {
                        id: ValueId::new(3),
                        ty: SsaType::Unit,
                        kind: InstructionKind::EndBorrow {
                            place: PlaceId::new(0),
                            loan: LoanId::new(0),
                            value: ValueId::new(1),
                        },
                        metadata: metadata_cleanup(EffectSet::PURE, 1),
                    },
                    drop_byte(4, 0, 0),
                    place_end(5, 0),
                ],
                terminator: Terminator::Return(ValueId::new(2)),
                metadata: block_metadata(),
            }],
            origin: Origin::SYNTHETIC,
        };
        function.blocks[0].instructions[3].metadata.failure_cleanup =
            Some(FailureCleanupId::new(0));
        function
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
