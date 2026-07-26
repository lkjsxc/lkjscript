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
    let index = instruction_operand(proto, instruction)?;
    let descriptor = chunk.enum_constructions.get(index).ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "enum constructor is missing",
        )
    })?;
    let definition = definition(chunk, descriptor.enum_id, proto, instruction)?;
    if definition.layout != descriptor.layout
        || usize::from(descriptor.substitution_arity)
            != usize::from(definition.type_parameter_count)
    {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "enum construction layout/substitution mismatch",
        ));
    }
    let variant = definition
        .variants
        .iter()
        .find(|item| item.id == descriptor.variant)
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "enum construction variant is missing",
            )
        })?;
    for _ in 0..variant.fields.len() {
        let _field = pop(state, proto, instruction)?;
    }
    state
        .stack
        .push(Kind::Enum(definition.id, Some(variant.id)));
    Ok(())
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
    if actual != Kind::Any && !matches!(actual, Kind::Enum(id, _) if id == definition.id) {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "enum identity mismatch",
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
    let active = match actual {
        Kind::Any => true,
        Kind::Enum(id, None) => id == definition.id,
        Kind::Enum(id, Some(active)) => id == definition.id && active == variant.id,
        _ => false,
    };
    if !active {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "inactive enum projection rejected before access",
        ));
    }
    state.stack.push(Kind::Any);
    Ok(())
}

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
