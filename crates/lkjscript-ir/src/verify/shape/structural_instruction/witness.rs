use crate::verify::*;
use crate::{EffectSet, Function, Instruction, InstructionKind, Program, SsaType};

pub(crate) fn verify(
    _program: &Program,
    function: &Function,
    instruction: &Instruction,
    types: &[SsaType],
) -> crate::Result<EffectSet> {
    let (parameter, operation) = match &instruction.kind {
        InstructionKind::MemoryWitnessIndependentOwner { parameter, .. } => (
            parameter,
            lkjscript_contracts::MemoryWitnessOperation::IndependentOwner,
        ),
        InstructionKind::MemoryWitnessCompare { parameter, .. } => (
            parameter,
            lkjscript_contracts::MemoryWitnessOperation::Compare,
        ),
        InstructionKind::MemoryWitnessDispose { parameter, .. } => (
            parameter,
            lkjscript_contracts::MemoryWitnessOperation::Dispose,
        ),
        _ => return fail("non-witness instruction reached memory witness verifier"),
    };
    let requirement = function
        .signature
        .memory_witness_parameters
        .iter()
        .find(|requirement| requirement.parameter == *parameter)
        .ok_or_else(|| {
            crate::IrError::new("SSA memory witness operation names no hidden parameter")
        })?;
    if !requirement.operations.contains(&operation) {
        return fail("SSA memory witness operation is not authorized");
    }
    let expected = SsaType::TypeParameter(parameter.clone());
    match &instruction.kind {
        InstructionKind::MemoryWitnessIndependentOwner { value, .. }
            if value_type(types, *value)? == &expected && instruction.ty == expected =>
        {
            Ok(EffectSet::ALLOCATES)
        }
        InstructionKind::MemoryWitnessCompare { left, right, .. }
            if value_type(types, *left)? == &expected
                && value_type(types, *right)? == &expected
                && instruction.ty == SsaType::Bool =>
        {
            Ok(EffectSet::READS_MEMORY)
        }
        InstructionKind::MemoryWitnessDispose { value, .. }
            if value_type(types, *value)? == &expected && instruction.ty == SsaType::Unit =>
        {
            Ok(EffectSet::PURE)
        }
        _ => fail("SSA memory witness operation has wrong operand or result type"),
    }
}
