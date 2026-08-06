use super::{instruction_error, types::*, Kind, OwnerIdentity, State};
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
                let callee_index = callee_proto;
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
                if callee_proto.arity != argc {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "statically known call arity mismatch",
                    ));
                }
                let memory_witnesses = validate_memory_witness_arguments(
                    chunk,
                    proto,
                    callee_index,
                    callee_proto,
                    &arguments,
                    instruction,
                )?;
                validate_resource_arguments(callee_proto, &arguments, proto, instruction)?;
                consume_resource_arguments(state, callee_proto, &arguments);
                validate_unique_arguments(callee_proto, &arguments, proto, instruction)?;
                validate_copy_arguments(callee_proto, &arguments, proto, instruction)?;
                validate_region_product_arguments(callee_proto, &arguments, proto, instruction)?;
                let structural_variables = validate_structural_arguments(
                    chunk,
                    callee_proto,
                    &arguments,
                    proto,
                    instruction,
                )?;
                call_return_kind(
                    chunk,
                    callee_proto,
                    instruction,
                    &structural_variables,
                    &memory_witnesses,
                )?
            } else {
                if arguments.iter().any(|kind| {
                    matches!(
                        kind,
                        Kind::Resource { .. }
                            | Kind::ResourceResult { .. }
                            | Kind::ByteVector(_)
                            | Kind::ByteSlice { .. }
                            | Kind::StructuralOwner { .. }
                            | Kind::StructuralOwnerRef { .. }
                            | Kind::StructuralView { .. }
                            | Kind::StructuralDestination { .. }
                            | Kind::RegionProduct(_)
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
            validate_resource_return(proto, returned, instruction, is_main)?;
            consume_resource_return(state, returned);
            validate_unique_exit_state(chunk, state, proto, instruction)?;
            validate_unique_return(proto, returned, instruction)?;
            validate_copy_return(proto, returned, instruction)?;
            validate_region_product_return(proto, returned, instruction)?;
            validate_structural_return(proto, returned, instruction)?;
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
include!("witness_arguments.rs");
include!("call_result.rs");
include!("copy_arguments.rs");
include!("region_products.rs");
include!("resource_arguments.rs");
include!("returns.rs");
