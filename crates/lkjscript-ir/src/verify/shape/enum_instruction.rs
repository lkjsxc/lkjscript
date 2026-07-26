use crate::verify::*;
use crate::{EffectSet, Instruction, InstructionKind, Program, SsaType, ValueId};

pub(super) fn verify(
    program: &Program,
    instruction: &Instruction,
    types: &[SsaType],
    kind: &InstructionKind,
) -> crate::Result<EffectSet> {
    match kind {
        InstructionKind::EnumValue {
            enum_id,
            variant,
            layout,
            fields,
        } => construction(
            program,
            instruction,
            types,
            *enum_id,
            *variant,
            *layout,
            fields,
        ),
        InstructionKind::EnumIsVariant {
            enum_id,
            variant,
            layout,
            value,
        } => variant_test(
            program,
            instruction,
            types,
            *enum_id,
            *variant,
            *layout,
            *value,
        ),
        InstructionKind::EnumField {
            enum_id,
            variant,
            field,
            layout,
            value,
        } => projection(
            program,
            instruction,
            types,
            (*enum_id, *variant, *field),
            *layout,
            *value,
        ),
        _ => fail("non-enum instruction reached enum verifier"),
    }
}

fn construction(
    program: &Program,
    instruction: &Instruction,
    types: &[SsaType],
    enum_id: crate::EnumId,
    variant: crate::VariantId,
    layout: crate::RuntimeLayoutId,
    fields: &[ValueId],
) -> crate::Result<EffectSet> {
    let definition = enum_by_id(program, enum_id)?;
    check_layout(definition, layout)?;
    let selected = variant_by_id(definition, variant)?;
    let SsaType::Enum { id, arguments } = &instruction.ty else {
        return fail("SSA enum construction result is not an enum type");
    };
    if *id != enum_id
        || arguments.len() != definition.type_parameters.len()
        || fields.len() != selected.fields.len()
    {
        return fail("SSA enum construction identity/substitution/field mismatch");
    }
    for (value, field) in fields.iter().zip(&selected.fields) {
        let expected = substitute(&field.ty, &definition.type_parameters, arguments);
        if value_type(types, *value)? != &expected {
            return fail("SSA enum construction field substitution/type mismatch");
        }
    }
    Ok(EffectSet::ALLOCATES)
}

fn variant_test(
    program: &Program,
    instruction: &Instruction,
    types: &[SsaType],
    enum_id: crate::EnumId,
    variant: crate::VariantId,
    layout: crate::RuntimeLayoutId,
    value: ValueId,
) -> crate::Result<EffectSet> {
    let definition = enum_by_id(program, enum_id)?;
    check_layout(definition, layout)?;
    let _selected = variant_by_id(definition, variant)?;
    match value_type(types, value)? {
        SsaType::Enum { id, arguments }
            if *id == enum_id
                && arguments.len() == definition.type_parameters.len()
                && instruction.ty == SsaType::Bool => {}
        _ => return fail("SSA enum variant test identity/type mismatch"),
    }
    Ok(EffectSet::READS_MEMORY)
}

fn projection(
    program: &Program,
    instruction: &Instruction,
    types: &[SsaType],
    ids: (crate::EnumId, crate::VariantId, crate::VariantFieldId),
    layout: crate::RuntimeLayoutId,
    value: ValueId,
) -> crate::Result<EffectSet> {
    let definition = enum_by_id(program, ids.0)?;
    check_layout(definition, layout)?;
    let selected = variant_by_id(definition, ids.1)?;
    let field = selected
        .fields
        .iter()
        .find(|candidate| candidate.id == ids.2)
        .ok_or_else(|| crate::IrError::new("SSA enum projection field/variant mismatch"))?;
    let SsaType::Enum { id, arguments } = value_type(types, value)? else {
        return fail("SSA enum projection input is not enum");
    };
    let expected = substitute(&field.ty, &definition.type_parameters, arguments);
    if *id != ids.0 || instruction.ty != expected {
        return fail("SSA enum projection identity/substitution/type mismatch");
    }
    Ok(EffectSet::READS_MEMORY)
}
