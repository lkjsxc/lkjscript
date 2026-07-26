use super::{instruction_error, types::*, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

pub(super) fn apply(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let op = instruction.op();
    match op {
        Op::MakeProduct => {
            let product_index = instruction_operand(proto, instruction)?;
            let product = chunk.products.get(product_index).ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "product metadata is missing",
                )
            })?;
            for _ in 0..product.fields.len() {
                let _field = pop(state, proto, instruction)?;
            }
            state.stack.push(Kind::Product(product.id));
        }
        Op::LoadProductField => {
            let descriptor = product_descriptor(chunk, proto, instruction)?;
            let product = pop(state, proto, instruction)?;
            expect_product(product, descriptor.product, proto, instruction)?;
            state.stack.push(Kind::Any);
        }
        Op::WithProductField => {
            let descriptor = product_descriptor(chunk, proto, instruction)?;
            let _replacement = pop(state, proto, instruction)?;
            let product = pop(state, proto, instruction)?;
            expect_product(product, descriptor.product, proto, instruction)?;
            state.stack.push(Kind::Product(descriptor.product));
        }
        Op::MakeEnum | Op::IsEnumVariant | Op::LoadEnumField => {
            return super::enums::apply(chunk, proto, instruction, state);
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}
