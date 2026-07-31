use super::{instruction_error, types::*, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

pub(super) fn apply(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    match instruction.op() {
        Op::MakeEnum => construction(chunk, proto, instruction, state),
        Op::IsEnumVariant => variant_test(chunk, proto, instruction, state),
        Op::LoadEnumField => projection(chunk, proto, instruction, state),
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
}

fn construction(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let _index = instruction_operand(proto, instruction)?;
    let _ = (chunk, state);
    Err(instruction_error(
        proto,
        instruction.op(),
        instruction.offset(),
        "enum construction is unsupported without structural metadata and operations",
    ))
}

fn variant_test(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let index = instruction_operand(proto, instruction)?;
    let descriptor = chunk.enum_variants.get(index).ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "enum variant is missing",
        )
    })?;
    let definition = definition(chunk, descriptor.enum_id, proto, instruction)?;
    if definition.layout != descriptor.layout
        || !definition
            .variants
            .iter()
            .any(|item| item.id == descriptor.variant)
    {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "enum variant descriptor mismatch",
        ));
    }
    let actual = pop(state, proto, instruction)?;
    if !matches!(
        actual,
        Kind::ResourceResult { .. } if definition.id.bytes() == crate::RESULT_ID
    ) {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "enum variant test is unsupported without structural metadata and operations",
        ));
    }
    state.stack.push(Kind::Bool);
    Ok(())
}

fn projection(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let index = instruction_operand(proto, instruction)?;
    let descriptor = chunk.enum_fields.get(index).ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "enum field is missing",
        )
    })?;
    let definition = definition(chunk, descriptor.enum_id, proto, instruction)?;
    let variant = definition
        .variants
        .iter()
        .find(|item| item.id == descriptor.variant)
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "enum field variant is missing",
            )
        })?;
    if definition.layout != descriptor.layout
        || !variant
            .fields
            .iter()
            .any(|field| field.id == descriptor.field)
    {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "enum field descriptor mismatch",
        ));
    }
    let actual = pop(state, proto, instruction)?;
    let active = matches!(
        actual,
        Kind::ResourceResult { .. } if definition.id.bytes() == crate::RESULT_ID
    );
    if !active {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "inactive enum projection rejected before access",
        ));
    }
    let projected = project_resource_result(state, actual, variant.id, descriptor.field);
    state.stack.push(projected);
    Ok(())
}

include!("types/enum_resources.rs");

fn definition<'a>(
    chunk: &'a Chunk,
    id: crate::EnumId,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<&'a crate::EnumMetadata> {
    chunk
        .enums
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "enum metadata is missing",
            )
        })
}
