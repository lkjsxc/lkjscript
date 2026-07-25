use super::instruction_error;
use crate::{Chunk, DecodedInstruction, FunctionProto, Result, StackEffect};

pub(super) fn stack(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<usize> {
    let op = instruction.op();
    match op.info().stack {
        StackEffect::Fixed { required, .. } => Ok(required),
        StackEffect::Call => instruction
            .operand()
            .map(usize::from)
            .and_then(|argc| argc.checked_add(1))
            .ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "call stack requirement overflow",
                )
            }),
        StackEffect::MakeProduct => instruction
            .operand()
            .map(usize::from)
            .and_then(|index| chunk.products.get(index))
            .map(|product| product.fields.len())
            .ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "product metadata is missing",
                )
            }),
    }
}
