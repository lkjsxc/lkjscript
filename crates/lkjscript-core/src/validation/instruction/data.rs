use super::{instruction_error, types::*, Kind, State};
use crate::{Chunk, Constant, DecodedInstruction, FunctionProto, Op, Result};

pub(super) fn apply(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let op = instruction.op();
    match op {
        Op::Nop | Op::Jump => {}
        Op::Trap => expect_pop(state, Kind::Str, proto, instruction)?,
        Op::LoadConst => {
            let constant = instruction
                .operand()
                .map(usize::from)
                .and_then(|index| chunk.constants.get(index))
                .ok_or_else(|| {
                    instruction_error(proto, op, instruction.offset(), "constant is missing")
                })?;
            state.stack.push(match constant {
                Constant::I64(_) => Kind::I64,
                Constant::F64(_) => Kind::F64,
                Constant::Str(_) => Kind::Str,
                Constant::Symbol(_) => Kind::Symbol,
                Constant::Proto(proto) => Kind::Proto(*proto),
            });
        }
        Op::LoadLocal => {
            let slot = instruction_operand(proto, instruction)?;
            let kind = state.locals.get(slot).copied().flatten().ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "local is not definitely initialized",
                )
            })?;
            state.stack.push(kind);
        }
        Op::StoreLocal => {
            let slot = instruction_operand(proto, instruction)?;
            let value = top(state, proto, instruction)?;
            let target = state.locals.get_mut(slot).ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "local index is out of range",
                )
            })?;
            *target = Some(value);
        }
        Op::LoadGlobal => {
            let slot = instruction_operand(proto, instruction)?;
            let kind = state.globals.get(slot).copied().flatten().ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "global is not definitely initialized",
                )
            })?;
            state.stack.push(kind);
        }
        Op::StoreGlobal => {
            let slot = instruction_operand(proto, instruction)?;
            let value = top(state, proto, instruction)?;
            let target = state.globals.get_mut(slot).ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "global index is out of range",
                )
            })?;
            *target = Some(value);
        }
        Op::Pop => {
            let _value = pop(state, proto, instruction)?;
        }
        Op::Dup => {
            let value = top(state, proto, instruction)?;
            state.stack.push(value);
        }
        Op::StdinHandle => state.stack.push(Kind::Handle),
        Op::False | Op::True => state.stack.push(Kind::Bool),
        Op::Unit => state.stack.push(Kind::Unit),
        Op::EmptyList => state.stack.push(Kind::List),
        Op::Argc => state.stack.push(Kind::I64),
        Op::EmptyStr => state.stack.push(Kind::Str),
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}
