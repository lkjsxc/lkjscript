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
        Op::Call => {
            let argc = instruction_operand(proto, instruction)?;
            let callee = pop(state, proto, instruction)?;
            let callee_proto = match callee {
                Kind::Closure(index) => Some(index),
                Kind::Any => None,
                _ => {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "call expects Closure",
                    ));
                }
            };
            for _ in 0..argc {
                let _argument = pop(state, proto, instruction)?;
            }
            if let Some(callee_proto) = callee_proto {
                let callee_proto = usize::try_from(callee_proto)
                    .ok()
                    .and_then(|index| chunk.protos.get(index))
                    .ok_or_else(|| {
                        instruction_error(
                            proto,
                            op,
                            instruction.offset(),
                            "closure prototype index is out of range",
                        )
                    })?;
                if usize::from(callee_proto.arity) != argc {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "statically known call arity mismatch",
                    ));
                }
            }
            state.stack.push(Kind::Any);
        }
        Op::Return => {
            if state.stack.len() != 1 {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "return requires exactly one operand value",
                ));
            }
            let _returned = pop(state, proto, instruction)?;
        }
        Op::MakeClosure => {
            let value = pop(state, proto, instruction)?;
            let Kind::Proto(index) = value else {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "MakeClosure expects a prototype constant",
                ));
            };
            if usize::try_from(index)
                .ok()
                .is_none_or(|index| index >= chunk.protos.len())
            {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "MakeClosure prototype is out of range",
                ));
            }
            state.stack.push(Kind::Closure(index));
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}
