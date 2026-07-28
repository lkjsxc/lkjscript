use super::{instruction_error, types::*, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

pub(super) fn apply(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
    is_main: bool,
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
            let mut arguments = Vec::with_capacity(argc);
            for _ in 0..argc {
                arguments.push(pop(state, proto, instruction)?);
            }
            arguments.reverse();
            let result = if let Some(callee_proto) = callee_proto {
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
                validate_resource_arguments(callee_proto, &arguments, proto, instruction)?;
                validate_unique_arguments(callee_proto, &arguments, proto, instruction)?;
                call_return_kind(callee_proto, instruction)?
            } else {
                if arguments.iter().any(|kind| {
                    matches!(
                        kind,
                        Kind::Resource(_)
                            | Kind::ResourceResult(_)
                            | Kind::ByteVector(_)
                            | Kind::ByteSlice { .. }
                    )
                }) {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "typed resources require statically known call metadata",
                    ));
                }
                Kind::Any
            };
            state.stack.push(result);
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
            let returned = pop(state, proto, instruction)?;
            validate_unique_exit_state(state, proto, instruction)?;
            validate_resource_return(proto, returned, instruction, is_main)?;
            validate_unique_return(proto, returned, instruction)?;
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

include!("arguments.rs");
include!("returns.rs");
