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
                resource_return_kind(callee_proto.return_resource)
            } else {
                if arguments
                    .iter()
                    .any(|kind| matches!(kind, Kind::Resource(_) | Kind::ResourceResult(_)))
                {
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
            validate_resource_return(proto, returned, instruction, is_main)?;
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

fn validate_resource_arguments(
    callee: &FunctionProto,
    arguments: &[Kind],
    caller: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    for (index, actual) in arguments.iter().copied().enumerate() {
        let expected = callee.parameter_resources.get(index).copied().flatten();
        match (expected, actual) {
            (Some(expected), Kind::Resource(actual)) if expected == actual => {}
            (Some(_), _) => {
                return Err(instruction_error(
                    caller,
                    instruction.op(),
                    instruction.offset(),
                    "typed resource call argument does not match declared kind",
                ));
            }
            (None, Kind::Resource(_) | Kind::ResourceResult(_)) => {
                return Err(instruction_error(
                    caller,
                    instruction.op(),
                    instruction.offset(),
                    "typed resource call argument lacks parameter metadata",
                ));
            }
            (None, _) => {}
        }
    }
    Ok(())
}

fn resource_return_kind(kind: Option<crate::ResourceReturnKind>) -> Kind {
    match kind {
        Some(crate::ResourceReturnKind::Resource(kind)) => Kind::Resource(kind),
        Some(crate::ResourceReturnKind::Result(kind)) => Kind::ResourceResult(kind),
        None => Kind::Any,
    }
}

fn validate_resource_return(
    proto: &FunctionProto,
    actual: Kind,
    instruction: DecodedInstruction,
    is_main: bool,
) -> Result<()> {
    if is_main && matches!(actual, Kind::Resource(_) | Kind::ResourceResult(_)) {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "typed resources cannot escape from main bytecode",
        ));
    }
    let expected = resource_return_kind(proto.return_resource);
    match (proto.return_resource, expected == actual, actual) {
        (Some(_), true, _) => Ok(()),
        (Some(_), false, _) => Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "typed resource return does not match declared kind",
        )),
        (None, _, Kind::Resource(_) | Kind::ResourceResult(_)) => Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "typed resource return lacks function metadata",
        )),
        (None, _, _) => Ok(()),
    }
}
