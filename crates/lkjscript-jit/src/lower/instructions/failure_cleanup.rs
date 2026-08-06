use super::*;

mod witness;
pub(super) use witness::call_result_witness_slot;

pub(super) fn lower_failure_cleanup(
    function: &Function,
    instruction: &Instruction,
    locals: &[LocalId],
    value_types: &[ValueType],
    layouts: &LayoutInterner,
) -> Result<Vec<lkjscript_native::FailureCleanupCall>, LoweringError> {
    let mut excluded = vec![instruction.id];
    excluded.extend(
        instruction
            .kind
            .operands()
            .into_iter()
            .filter(|value| consuming_operand(&instruction.kind, *value)),
    );
    lower_failure_cleanup_id(
        function,
        instruction.metadata.failure_cleanup,
        locals,
        value_types,
        layouts,
        &excluded,
    )
}

pub(in crate::lower) fn lower_failure_cleanup_id(
    function: &Function,
    cleanup: Option<lkjscript_ir::FailureCleanupRoots>,
    locals: &[LocalId],
    value_types: &[ValueType],
    layouts: &LayoutInterner,
    excluded: &[ValueId],
) -> Result<Vec<lkjscript_native::FailureCleanupCall>, LoweringError> {
    let Some(cleanup) = cleanup else {
        return Ok(Vec::new());
    };
    if cleanup.ids().any(|root| {
        root.index()
            .is_none_or(|index| index >= function.failure_cleanups.len())
    }) {
        return Err(LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function.id),
            "SSA native failure cleanup references an invalid chain",
        ));
    }
    let excluded: HashSet<_> = excluded.iter().copied().collect();
    function
        .failure_cleanup_actions(Some(cleanup))
        .filter(|action| {
            let value = match action {
                lkjscript_ir::FailureCleanupAction::EndBorrow { value, .. }
                | lkjscript_ir::FailureCleanupAction::DropOwner { value, .. } => *value,
            };
            !excluded.contains(&value)
        })
        .map(|action| {
            let (value, runtime, structural) = match action {
                lkjscript_ir::FailureCleanupAction::EndBorrow { value, .. } => {
                    match value_type(value_types, *value)? {
                        ValueType::Loan(LoanType::Bytes) => {
                            (*value, Some(RuntimeCallSlot::BytesEndBorrow), None)
                        }
                        ValueType::Loan(LoanType::ByteSlice) => {
                            (*value, Some(RuntimeCallSlot::ByteSliceEnd), None)
                        }
                        ValueType::Loan(LoanType::ByteSliceMut) => {
                            (*value, Some(RuntimeCallSlot::ByteSliceMutEnd), None)
                        }
                        ValueType::StructuralView(view) => (
                            *value,
                            None,
                            Some(lkjscript_native::StructuralOperation::EndView(view)),
                        ),
                        _ => {
                            return Err(LoweringError::new(
                                LoweringFailureCode::InvalidFunction,
                                Some(function.id),
                                "SSA native failure loan has an invalid type",
                            ))
                        }
                    }
                }
                lkjscript_ir::FailureCleanupAction::DropOwner {
                    value,
                    glue: lkjscript_ir::DropGlueIdentity::ByteVector,
                    ..
                } => (*value, Some(RuntimeCallSlot::ByteVectorDrop), None),
                lkjscript_ir::FailureCleanupAction::DropOwner {
                    value,
                    glue: lkjscript_ir::DropGlueIdentity::Bytes,
                    ..
                } => (*value, Some(RuntimeCallSlot::BytesDrop), None),
                lkjscript_ir::FailureCleanupAction::DropOwner {
                    glue: lkjscript_ir::DropGlueIdentity::Resource(_),
                    ..
                } => return unsupported_operation(function.id, "owned resource failure cleanup"),
                lkjscript_ir::FailureCleanupAction::DropOwner {
                    value,
                    glue: lkjscript_ir::DropGlueIdentity::Structural(_),
                    ..
                } => {
                    let operation = match value_type(value_types, *value)? {
                        ValueType::StructuralOwner(value_type) => {
                            lkjscript_native::StructuralOperation::Drop(value_type)
                        }
                        ValueType::StructuralKey => {
                            lkjscript_native::StructuralOperation::WitnessDisposeStatic(
                                call_result_witness_slot(function, *value, layouts)?,
                            )
                        }
                        ValueType::StructuralDestination(_) => {
                            let (aggregate, storage, initialized) =
                                layouts.structural().destination(function, *value)?;
                            lkjscript_native::StructuralOperation::DestinationAbort {
                                aggregate,
                                storage,
                                initialized,
                            }
                        }
                        actual => {
                            return Err(LoweringError::new(
                                LoweringFailureCode::InvalidFunction,
                                Some(function.id),
                                format!(
                                    "SSA structural failure owner {} has invalid type {actual:?}",
                                    value.raw(),
                                ),
                            ));
                        }
                    };
                    (*value, None, Some(operation))
                }
            };
            let local = value_local(locals, value, function.id)?;
            if let Some(slot) = runtime {
                return Ok(lkjscript_native::FailureCleanupCall::new(slot, local));
            }
            let operation = structural.ok_or_else(|| {
                LoweringError::new(
                    LoweringFailureCode::InvalidFunction,
                    Some(function.id),
                    "SSA failure cleanup operation is absent",
                )
            })?;
            let descriptor = lkjscript_native::StructuralCallDescriptor::new(operation.clone())
                .map_err(|error| {
                    LoweringError::new(
                        LoweringFailureCode::Backend,
                        Some(function.id),
                        format!("failure cleanup {operation:?}: {error}"),
                    )
                })?;
            Ok(lkjscript_native::FailureCleanupCall::structural(
                descriptor, local,
            ))
        })
        .collect()
}
