use super::{types::*, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result, StructuralSliceExt};

pub(super) fn apply(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let op = instruction.op();
    match op {
        Op::Print => {
            let _value = pop(state, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Stdio, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::Flush => {
            expect_capability(state, crate::CapabilityKind::Stdio, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::ReadByte => {
            expect_capability(state, crate::CapabilityKind::Stdio, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::WriteByte => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Stdio, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::Exit => {
            expect_pop(state, Kind::I64, proto, instruction)?;
        }
        Op::WriteStr => {
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::String,
                Kind::Str,
                proto,
                instruction,
            )?;
            expect_capability(state, crate::CapabilityKind::Stdio, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::Arg => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Arguments, proto, instruction)?;
            state
                .stack
                .push(structural_string_option(chunk, proto, instruction)?);
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}

fn structural_string_option(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    if chunk.memory_plan.is_none() {
        return Ok(option_kind());
    }
    let type_id = chunk.structural_types.iter().find_map(|ty| {
        let crate::StructuralTypeKind::Enum(enum_id) = ty.kind else {
            return None;
        };
        if enum_id.bytes() != crate::OPTION_ID {
            return None;
        }
        let layout = chunk.structural_layouts.get_structural(ty.layout)?;
        let crate::StructuralLayoutKind::Enum { variants, .. } = &layout.kind else {
            return None;
        };
        variants
            .iter()
            .flat_map(|variant| &variant.fields)
            .any(|field| {
                field
                    .runtime_type
                    .is_some_and(|ty| ty.kind == crate::StructuralKind::String)
            })
            .then_some(ty.id)
    });
    let representation = type_id.and_then(|type_id| {
        chunk.structural_representations.iter().find(|item| {
            item.type_id == type_id && item.category == crate::StructuralValueCategory::Owner
        })
    });
    let representation = representation.ok_or_else(|| {
        super::instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "argument result lacks exact option-string structural metadata",
        )
    })?;
    Ok(Kind::StructuralOwner {
        representation: representation.id,
        owner: super::bytes::new_owner(instruction)?,
        active_variant: None,
    })
}

fn expect_capability(
    state: &mut State,
    kind: crate::CapabilityKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    expect_pop(state, Kind::Capability(kind), proto, instruction)
}
