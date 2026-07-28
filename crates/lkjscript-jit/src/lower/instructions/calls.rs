use super::*;

pub(super) fn indirect_call(function: FunctionId) -> Result<(), LoweringError> {
    Err(LoweringError::new(
        LoweringFailureCode::IndirectCall,
        Some(function),
        "indirect native calls are unsupported",
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_direct_call(
    function: &Function,
    instruction: &Instruction,
    callee: FunctionId,
    arguments: &[ValueId],
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    native_functions: &[(FunctionId, lkjscript_native::FunctionId)],
    builder: &mut FunctionBuilder,
) -> Result<Result<lkjscript_native::ValueId, lkjscript_native::PlanError>, LoweringError> {
    let unentered = lower_unentered_cleanup(function, arguments, locals, value_types)?;
    let poll = builder
        .runtime_call(block, RuntimeCallSlot::Poll, Vec::new())
        .map_err(LoweringError::backend)?;
    let mut poll_cleanup = unentered.clone();
    poll_cleanup.extend(lower_failure_cleanup(
        function,
        instruction,
        locals,
        value_types,
    )?);
    builder
        .set_instruction_failure_cleanup(poll, poll_cleanup)
        .map_err(LoweringError::backend)?;
    let arguments = read_values(builder, block, locals, arguments, function.id)?;
    let callee = native_function(native_functions, callee)?;
    let call = builder
        .call(block, callee, arguments)
        .map_err(LoweringError::backend)?;
    builder
        .set_instruction_unentered_cleanup(call, unentered)
        .map_err(LoweringError::backend)?;
    Ok(Ok(call))
}

fn lower_unentered_cleanup(
    function: &Function,
    arguments: &[ValueId],
    locals: &[LocalId],
    value_types: &[ValueType],
) -> Result<Vec<lkjscript_native::FailureCleanupCall>, LoweringError> {
    let mut cleanup = Vec::new();
    for value in arguments.iter().rev() {
        let slot = match value_type(value_types, *value)? {
            ValueType::Unique(lkjscript_native::UniqueType::ByteVector) => {
                Some(RuntimeCallSlot::ByteVectorDrop)
            }
            ValueType::Unique(lkjscript_native::UniqueType::Bytes) => {
                Some(RuntimeCallSlot::BytesDrop)
            }
            _ => None,
        };
        if let Some(slot) = slot {
            cleanup.push(lkjscript_native::FailureCleanupCall::new(
                slot,
                value_local(locals, *value, function.id)?,
            ));
        }
    }
    Ok(cleanup)
}
