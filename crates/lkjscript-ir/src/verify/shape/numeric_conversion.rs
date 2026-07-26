use crate::numeric_contract::{NumericError, ERROR_ID, ERROR_LAYOUT};
use crate::verify::*;
use crate::{EffectSet, Instruction, InstructionKind, Program, SsaType};

pub(super) fn verify(
    program: &Program,
    instruction: &Instruction,
    types: &[SsaType],
) -> crate::Result<EffectSet> {
    verify_numeric_error(program)?;
    let (value, input, output, effects) = match &instruction.kind {
        InstructionKind::F64FromI64Exact { value } => (
            *value,
            SsaType::I64,
            numeric_result(SsaType::F64),
            EffectSet::ALLOCATES,
        ),
        InstructionKind::F64FromI64Rounded { value } => {
            (*value, SsaType::I64, SsaType::F64, EffectSet::PURE)
        }
        InstructionKind::I64FromF64Exact { value } | InstructionKind::I64FromF64Trunc { value } => {
            (
                *value,
                SsaType::F64,
                numeric_result(SsaType::I64),
                EffectSet::ALLOCATES,
            )
        }
        _ => return fail("non-conversion reached numeric conversion verifier"),
    };
    if value_type(types, value)? != &input || instruction.ty != output {
        return fail("numeric conversion operand or result type mismatch");
    }
    Ok(effects)
}

fn numeric_result(ok: SsaType) -> SsaType {
    crate::prelude_contract::result(
        ok,
        SsaType::Enum {
            id: crate::EnumId::new(ERROR_ID),
            arguments: Vec::new(),
        },
    )
}

fn verify_numeric_error(program: &Program) -> crate::Result<()> {
    let definition = program
        .enums
        .iter()
        .find(|item| item.id == crate::EnumId::new(ERROR_ID))
        .ok_or_else(|| crate::IrError::new("NumericError prelude metadata is missing"))?;
    if !definition.type_parameters.is_empty()
        || definition.layout.identity != crate::RuntimeLayoutId::new(ERROR_LAYOUT)
        || definition.layout.recursive
        || definition.variants.len() != 4
    {
        return fail("NumericError prelude metadata is malformed");
    }
    let cases = [
        NumericError::NonFinite,
        NumericError::OutOfRange,
        NumericError::Fractional,
        NumericError::Inexact,
    ];
    for error in cases {
        let variant = definition
            .variants
            .iter()
            .find(|variant| variant.id == crate::VariantId::new(error.variant_id()))
            .ok_or_else(|| crate::IrError::new("NumericError case identity is missing"))?;
        if variant.physical_tag != error.physical_tag() || !variant.fields.is_empty() {
            return fail("NumericError case metadata is malformed");
        }
    }
    Ok(())
}
