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
                Constant::StaticBytes(_) => Kind::StaticBytes,
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
            if is_unique(kind) {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "unique owners/views require typed local opcodes",
                ));
            }
            state.stack.push(kind);
        }
        Op::StoreLocal => {
            let slot = instruction_operand(proto, instruction)?;
            let value = top(state, proto, instruction)?;
            if is_unique(value) {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "unique owners/views require typed local opcodes",
                ));
            }
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
            if let Some(expected) = chunk.global_prototypes.get(slot).copied().flatten() {
                if value != Kind::Closure(expected) {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "global closure does not match declared prototype metadata",
                    ));
                }
            } else if matches!(value, Kind::Resource(_) | Kind::ResourceResult(_))
                || is_unique(value)
            {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "typed resources cannot be stored in bytecode globals",
                ));
            }
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
            let value = pop(state, proto, instruction)?;
            if is_unique(value) {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "Pop cannot erase a unique owner or live view",
                ));
            }
        }
        Op::Dup => {
            let value = top(state, proto, instruction)?;
            if is_unique(value) {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "Dup cannot forge a unique owner or byte view",
                ));
            }
            state.stack.push(value);
        }
        Op::StdinHandle => {
            expect_pop(
                state,
                Kind::Capability(crate::CapabilityKind::Stdio),
                proto,
                instruction,
            )?;
            state
                .stack
                .push(Kind::Resource(crate::ResourceKind::InputStream));
        }
        Op::False | Op::True => state.stack.push(Kind::Bool),
        Op::Unit => state.stack.push(Kind::Unit),
        Op::EmptyList => state.stack.push(Kind::List),
        Op::Argc => {
            expect_pop(
                state,
                Kind::Capability(crate::CapabilityKind::Arguments),
                proto,
                instruction,
            )?;
            state.stack.push(Kind::I64);
        }
        Op::EmptyStr => state.stack.push(Kind::Str),
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}

fn is_unique(kind: Kind) -> bool {
    matches!(kind, Kind::ByteVector(_) | Kind::ByteSlice { .. })
}
