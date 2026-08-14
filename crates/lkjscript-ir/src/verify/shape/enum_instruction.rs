use crate::verify::*;
use crate::{EffectSet, Instruction, InstructionKind, Program, SsaType, ValueId};

pub(super) fn verify(
    program: &Program,
    instruction: &Instruction,
    types: &[SsaType],
    kind: &InstructionKind,
) -> crate::Result<EffectSet> {
    match kind {
        InstructionKind::EnumValue { .. } => {
            fail("SSA enum construction is unsupported without structural metadata and operations")
        }
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
            field_index,
            layout,
            value,
        } => projection(
            program,
            instruction,
            types,
            (*enum_id, *variant, *field),
            *field_index,
            *layout,
            *value,
        ),
        _ => fail("non-enum instruction reached enum verifier"),
    }
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
    if instruction.ty != SsaType::Bool || !is_resource_result(value_type(types, value)?, enum_id) {
        return fail("SSA resource-result variant test identity/type mismatch");
    }
    Ok(EffectSet::READS_MEMORY)
}

fn projection(
    program: &Program,
    instruction: &Instruction,
    types: &[SsaType],
    ids: (crate::EnumId, crate::VariantId, crate::VariantFieldId),
    field_index: u64,
    layout: crate::RuntimeLayoutId,
    value: ValueId,
) -> crate::Result<EffectSet> {
    let definition = enum_by_id(program, ids.0)?;
    check_layout(definition, layout)?;
    let selected = variant_by_id(definition, ids.1)?;
    let field = usize::try_from(field_index)
        .ok()
        .and_then(|index| selected.fields.get(index))
        .filter(|candidate| candidate.id == ids.2)
        .ok_or_else(|| crate::IrError::new("SSA enum projection field index/identity mismatch"))?;
    let input = value_type(types, value)?;
    let SsaType::Enum { arguments, .. } = input else {
        return fail("SSA resource-result projection input is not enum");
    };
    let expected = substitute(&field.ty, &definition.type_parameters, arguments)?;
    if instruction.ty != expected {
        return fail("SSA enum projection identity/substitution/type mismatch");
    }
    if program.memory.type_for(input).is_some() {
        Ok(EffectSet::READS_MEMORY.union(EffectSet::ALLOCATES))
    } else if is_resource_result(input, ids.0) {
        Ok(EffectSet::READS_MEMORY)
    } else {
        fail("SSA enum projection lacks structural or resource-adapter metadata")
    }
}

fn is_resource_result(ty: &SsaType, enum_id: crate::EnumId) -> bool {
    matches!(
        ty,
        SsaType::Enum { id, arguments }
            if enum_id.bytes() == crate::prelude_contract::RESULT_ID
                && id.bytes() == crate::prelude_contract::RESULT_ID
                && matches!(arguments.as_slice(), [SsaType::Resource(_), _])
    )
}
