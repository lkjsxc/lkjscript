use crate::codegen::*;

pub(super) fn structural_routes(program: &lkjscript_ir::Program) -> bool {
    program.functions.iter().any(|function| {
        function.blocks.iter().any(|block| {
            block.instructions.iter().any(|instruction| {
                matches!(
                    instruction.kind,
                    InstructionKind::StructuralPublish { .. }
                        | InstructionKind::DestinationCreate { .. }
                        | InstructionKind::DestinationFieldInit { .. }
                        | InstructionKind::DestinationFinish { .. }
                        | InstructionKind::DestinationAbort { .. }
                        | InstructionKind::AggregateFieldBorrow { .. }
                        | InstructionKind::AggregateTag { .. }
                        | InstructionKind::AggregateConsumePayload { .. }
                        | InstructionKind::StringUtf8View { .. }
                )
            })
        })
    })
}

pub(super) fn executable_witnesses(
    program: &lkjscript_ir::Program,
    structural_routes: bool,
) -> bool {
    structural_routes
        || program.functions.iter().any(|function| {
            function.blocks.iter().any(|block| {
                block.instructions.iter().any(|instruction| {
                    matches!(
                        &instruction.kind,
                        InstructionKind::Call {
                            instantiation: Some(instantiation),
                            ..
                        } if !instantiation.memory_witnesses.is_empty()
                    )
                })
            })
        })
}
