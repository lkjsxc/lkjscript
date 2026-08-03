pub(super) fn lower_call_result_dispose(
    function: &Function,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let locator = crate::lower::instructions::failure_cleanup::call_result_witness_slot(
        function, value, layouts,
    )?;
    let locator = builder
        .memory_witness_locator(block, locator)
        .map_err(LoweringError::backend)?;
    let key = read_value(builder, block, locals, value, function.id)?;
    let descriptor = lkjscript_native::StructuralCallDescriptor::new(
        lkjscript_native::StructuralOperation::WitnessDispose,
    )
    .map_err(LoweringError::backend)?;
    builder
        .structural_call(block, descriptor, vec![locator, key])
        .map_err(|error| {
            LoweringError::new(
                LoweringFailureCode::Backend,
                Some(function.id),
                format!("dynamic call result dispose: {error}"),
            )
        })
}

pub(super) fn lower_witness_operation(
    function: &Function,
    instruction: &Instruction,
    parameter: &str,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let witness = function
        .signature
        .memory_witness_parameters
        .iter()
        .position(|requirement| requirement.parameter == parameter)
        .ok_or_else(|| {
            LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function.id),
                "native witness operation has no hidden parameter",
            )
        })?;
    let hidden = function
        .signature
        .parameters
        .len()
        .checked_add(witness)
        .ok_or_else(|| invalid_structural("native hidden witness parameter overflow"))?;
    let locator = builder.parameter(hidden).map_err(LoweringError::backend)?;
    let (operation, mut arguments) = match &instruction.kind {
        InstructionKind::MemoryWitnessIndependentOwner { value, .. } => (
            lkjscript_native::StructuralOperation::WitnessIndependentOwner,
            vec![observe_value(function, *value, block, locals, builder)?],
        ),
        InstructionKind::MemoryWitnessCompare { left, right, .. } => {
            let left_value = read_value(builder, block, locals, *left, function.id)?;
            let right_value = if left == right {
                left_value
            } else {
                read_value(builder, block, locals, *right, function.id)?
            };
            (
                lkjscript_native::StructuralOperation::WitnessCompare,
                vec![left_value, right_value],
            )
        }
        InstructionKind::MemoryWitnessDispose { value, .. } => (
            lkjscript_native::StructuralOperation::WitnessDispose,
            vec![read_value(builder, block, locals, *value, function.id)?],
        ),
        _ => return Err(invalid_structural("non-witness instruction reached witness lowering")),
    };
    arguments.insert(0, locator);
    let descriptor = lkjscript_native::StructuralCallDescriptor::new(operation)
        .map_err(LoweringError::backend)?;
    builder
        .structural_call(block, descriptor, arguments)
        .map_err(|error| {
            LoweringError::new(
                LoweringFailureCode::Backend,
                Some(function.id),
                format!("dynamic witness operation: {error}"),
            )
        })
}
