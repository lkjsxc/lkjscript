use crate::verify::*;
use crate::{EffectSet, Function, Instruction, InstructionKind, Program, SsaType};

pub(crate) fn verify(
    _program: &Program,
    function: &Function,
    instruction: &Instruction,
    types: &[SsaType],
) -> crate::Result<EffectSet> {
    let (parameter, value, operation) = match &instruction.kind {
        InstructionKind::MemoryWitnessIndependentOwner { parameter, value } => (
            parameter,
            value,
            lkjscript_contracts::MemoryWitnessOperation::IndependentOwner,
        ),
        InstructionKind::MemoryWitnessDispose { parameter, value } => (
            parameter,
            value,
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
    if !requirement.operations.contains(&operation)
        || value_type(types, *value)? != &SsaType::TypeParameter(parameter.clone())
    {
        return fail("SSA memory witness operation has wrong parameter or operand type");
    }
    match instruction.kind {
        InstructionKind::MemoryWitnessIndependentOwner { .. }
            if instruction.ty == SsaType::TypeParameter(parameter.clone()) =>
        {
            Ok(EffectSet::ALLOCATES)
        }
        InstructionKind::MemoryWitnessDispose { .. } if instruction.ty == SsaType::Unit => {
            Ok(EffectSet::PURE)
        }
        _ => fail("SSA memory witness operation has wrong result type"),
    }
}
