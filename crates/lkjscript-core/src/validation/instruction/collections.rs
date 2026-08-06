use super::{instruction_error, types::*, Kind, OwnerIdentity, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

pub(super) fn apply(
    _chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let op = instruction.op();
    match op {
        Op::Cons => {
            expect_pop(state, Kind::List, proto, instruction)?;
            let _car = pop(state, proto, instruction)?;
            state.stack.push(Kind::List);
        }
        Op::Car => {
            expect_pop(state, Kind::List, proto, instruction)?;
            let representation = instruction.operand().index().ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "list-first element representation is missing",
                )
            })?;
            if representation == usize::from(u16::MAX) {
                state.stack.push(Kind::Any);
            } else {
                let representation = crate::StructuralRepresentationId::new(
                    u16::try_from(representation).map_err(|_| {
                        instruction_error(
                            proto,
                            op,
                            instruction.offset(),
                            "list-first representation exceeds u16",
                        )
                    })?,
                );
                let metadata = _chunk
                    .structural_representations
                    .get(representation.index())
                    .filter(|metadata| metadata.id == representation)
                    .ok_or_else(|| {
                        instruction_error(
                            proto,
                            op,
                            instruction.offset(),
                            "list-first element representation is stale",
                        )
                    })?;
                if metadata.category != crate::StructuralValueCategory::Owner {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "list-first requires an owner element representation",
                    ));
                }
                let owner = OwnerIdentity::instruction(instruction.offset(), 1);
                state.stack.push(Kind::StructuralOwner {
                    representation,
                    owner,
                    active_variant: None,
                });
            }
        }
        Op::Cdr => {
            expect_pop(state, Kind::List, proto, instruction)?;
            state.stack.push(Kind::List);
        }
        Op::IsEmptyList => {
            expect_pop(state, Kind::List, proto, instruction)?;
            state.stack.push(Kind::Bool);
        }
        Op::SameObject => {
            let right = pop(state, proto, instruction)?;
            let left = pop(state, proto, instruction)?;
            if left != Kind::Any && right != Kind::Any {
                let valid = left == right && matches!(left, Kind::Resource { .. });
                if !valid {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "is-same-object expects matching resource categories",
                    ));
                }
            }
            state.stack.push(Kind::Bool);
        }
        Op::ListEqual => {
            expect_pop(state, Kind::List, proto, instruction)?;
            expect_pop(state, Kind::List, proto, instruction)?;
            state.stack.push(Kind::Bool);
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}
