use super::{instruction_error, types::*, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

pub(super) fn apply(
    _chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let op = instruction.op();
    match op {
        Op::Add | Op::Sub | Op::Mul | Op::Div => {
            let right = pop(state, proto, instruction)?;
            let left = pop(state, proto, instruction)?;
            expect_numeric(left, proto, instruction)?;
            expect_numeric(right, proto, instruction)?;
            state
                .stack
                .push(if left == Kind::Any || right == Kind::Any {
                    Kind::Any
                } else if left == Kind::F64 || right == Kind::F64 {
                    Kind::F64
                } else {
                    Kind::I64
                });
        }
        Op::Lt | Op::Le | Op::Gt | Op::Ge => {
            expect_two_numeric(state, proto, instruction)?;
            state.stack.push(Kind::Bool);
        }
        Op::BitAnd | Op::BitOr | Op::BitXor => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::EqualValue => {
            let right = pop(state, proto, instruction)?;
            let left = pop(state, proto, instruction)?;
            if left != Kind::Any
                && right != Kind::Any
                && (left != right || !is_value_comparable(left))
            {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "incompatible equal-value categories",
                ));
            }
            state.stack.push(Kind::Bool);
        }
        Op::Not | Op::JumpIfFalse => {
            expect_pop(state, Kind::Bool, proto, instruction)?;
            if op == Op::Not {
                state.stack.push(Kind::Bool);
            }
        }
        Op::F64BitsEqual => {
            expect_pop(state, Kind::F64, proto, instruction)?;
            expect_pop(state, Kind::F64, proto, instruction)?;
            state.stack.push(Kind::Bool);
        }
        Op::F64FromI64Exact => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state
                .stack
                .push(Kind::Enum(crate::EnumId::new(crate::RESULT_ID), None));
        }
        Op::F64FromI64Rounded => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::F64);
        }
        Op::I64FromF64Exact | Op::I64FromF64Trunc => {
            expect_pop(state, Kind::F64, proto, instruction)?;
            state
                .stack
                .push(Kind::Enum(crate::EnumId::new(crate::RESULT_ID), None));
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}
