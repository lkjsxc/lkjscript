use super::*;

pub(super) fn verify_failure_cleanup(
    function: &FunctionPlan,
    instruction: &crate::plan::Instruction,
    initialized_locals: &[bool],
) -> Result<(), VerificationError> {
    verify_cleanup_calls(function, &instruction.failure_cleanup, initialized_locals)?;
    if !instruction.unentered_cleanup.is_empty()
        && !matches!(&instruction.operation, crate::plan::Operation::Call(_, _))
    {
        return Err(VerificationError::TypeMismatch(
            "unentered cleanup on non-call",
        ));
    }
    verify_cleanup_calls(function, &instruction.unentered_cleanup, initialized_locals)
}

fn verify_cleanup_calls(
    function: &FunctionPlan,
    cleanups: &[crate::plan::FailureCleanupCall],
    initialized_locals: &[bool],
) -> Result<(), VerificationError> {
    let mut seen = HashSet::new();
    for cleanup in cleanups {
        let index = local_index(function, cleanup.local)?;
        if !initialized_locals.get(index).copied().unwrap_or(false) || !seen.insert(cleanup.local) {
            return Err(VerificationError::LocalNotInitialized(cleanup.local));
        }
        let signature = cleanup
            .slot
            .plan_signature()
            .ok_or(VerificationError::TypeMismatch(
                "failure cleanup runtime slot",
            ))?;
        if signature.parameters() != [function.locals[index].value_type]
            || signature.result() != ValueType::Unit
            || !matches!(
                cleanup.slot,
                RuntimeCallSlot::ByteSliceEnd
                    | RuntimeCallSlot::ByteSliceMutEnd
                    | RuntimeCallSlot::ByteVectorDrop
                    | RuntimeCallSlot::BytesEndBorrow
                    | RuntimeCallSlot::BytesDrop
            )
        {
            return Err(VerificationError::TypeMismatch(
                "failure cleanup runtime call",
            ));
        }
    }
    Ok(())
}
