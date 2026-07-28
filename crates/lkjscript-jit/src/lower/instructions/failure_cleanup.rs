use super::*;

pub(super) fn lower_failure_cleanup(
    function: &Function,
    instruction: &Instruction,
    locals: &[LocalId],
    value_types: &[ValueType],
) -> Result<Vec<lkjscript_native::FailureCleanupCall>, LoweringError> {
    lower_failure_cleanup_id(
        function,
        instruction.metadata.failure_cleanup,
        locals,
        value_types,
    )
}

pub(in crate::lower) fn lower_failure_cleanup_id(
    function: &Function,
    cleanup: Option<lkjscript_ir::FailureCleanupId>,
    locals: &[LocalId],
    value_types: &[ValueType],
) -> Result<Vec<lkjscript_native::FailureCleanupCall>, LoweringError> {
    let Some(cleanup) = cleanup else {
        return Ok(Vec::new());
    };
    let plan = function
        .failure_cleanups
        .get(cleanup.index().unwrap_or(usize::MAX))
        .filter(|plan| plan.id == cleanup)
        .ok_or_else(|| {
            LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function.id),
                "SSA native failure cleanup references an invalid plan",
            )
        })?;
    plan.actions
        .iter()
        .map(|action| {
            let (value, slot) = match action {
                lkjscript_ir::FailureCleanupAction::EndBorrow { value, .. } => {
                    let slot = match value_type(value_types, *value)? {
                        ValueType::Loan(LoanType::Bytes) => RuntimeCallSlot::BytesEndBorrow,
                        ValueType::Loan(LoanType::ByteSlice) => RuntimeCallSlot::ByteSliceEnd,
                        ValueType::Loan(LoanType::ByteSliceMut) => RuntimeCallSlot::ByteSliceMutEnd,
                        _ => {
                            return Err(LoweringError::new(
                                LoweringFailureCode::InvalidFunction,
                                Some(function.id),
                                "SSA native failure loan has an invalid type",
                            ))
                        }
                    };
                    (*value, slot)
                }
                lkjscript_ir::FailureCleanupAction::DropOwner {
                    value,
                    glue: lkjscript_ir::DropGlueIdentity::ByteVector,
                    ..
                } => (*value, RuntimeCallSlot::ByteVectorDrop),
                lkjscript_ir::FailureCleanupAction::DropOwner {
                    value,
                    glue: lkjscript_ir::DropGlueIdentity::Bytes,
                    ..
                } => (*value, RuntimeCallSlot::BytesDrop),
                lkjscript_ir::FailureCleanupAction::DropOwner {
                    glue: lkjscript_ir::DropGlueIdentity::Resource(_),
                    ..
                } => return unsupported_operation(function.id, "owned resource failure cleanup"),
            };
            Ok(lkjscript_native::FailureCleanupCall::new(
                slot,
                value_local(locals, value, function.id)?,
            ))
        })
        .collect()
}
