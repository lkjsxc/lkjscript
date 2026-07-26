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
        Op::OkWrap | Op::ErrWrap => {
            let _value = pop(state, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::IsOk => {
            let actual = pop(state, proto, instruction)?;
            if !matches!(
                actual,
                Kind::Any | Kind::Result | Kind::NumericResultF64 | Kind::NumericResultI64
            ) {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "is-ok expects Result",
                ));
            }
            state.stack.push(Kind::Bool);
        }
        Op::UnwrapOk | Op::UnwrapErr => {
            let actual = pop(state, proto, instruction)?;
            let result = match (op, actual) {
                (_, Kind::Any | Kind::Result) => Kind::Any,
                (Op::UnwrapOk, Kind::NumericResultF64) => Kind::F64,
                (Op::UnwrapOk, Kind::NumericResultI64) => Kind::I64,
                (Op::UnwrapErr, Kind::NumericResultF64 | Kind::NumericResultI64) => {
                    Kind::Enum(crate::EnumId::new(crate::NUMERIC_ERROR_ID), None)
                }
                _ => {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "unwrap expects Result",
                    ));
                }
            };
            state.stack.push(result);
        }
        Op::SomeWrap => {
            let _value = pop(state, proto, instruction)?;
            state.stack.push(Kind::Option);
        }
        Op::IsSome => {
            expect_pop(state, Kind::Option, proto, instruction)?;
            state.stack.push(Kind::Bool);
        }
        Op::UnwrapSome => {
            expect_pop(state, Kind::Option, proto, instruction)?;
            state.stack.push(Kind::Any);
        }
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
