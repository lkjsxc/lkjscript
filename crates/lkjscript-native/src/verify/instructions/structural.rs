use super::*;

pub(super) fn verify_structural_call(
    function: &FunctionPlan,
    instruction: &crate::plan::Instruction,
    descriptor: &StructuralCallDescriptor,
    arguments: &[ValueId],
) -> Result<(), VerificationError> {
    if !descriptor.canonical() {
        return Err(VerificationError::TypeMismatch("structural runtime call"));
    }
    verify_arguments(
        function,
        arguments,
        descriptor.signature(),
        "structural runtime call",
    )?;
    require_output(
        instruction,
        descriptor.signature().result(),
        "structural runtime call",
    )
}
