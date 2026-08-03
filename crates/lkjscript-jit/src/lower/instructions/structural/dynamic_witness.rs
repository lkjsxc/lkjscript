pub(super) fn lower_call_result_dispose(
    program: &lkjscript_ir::Program,
    function: &Function,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let call = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find(|instruction| instruction.id == value)
        .ok_or_else(|| invalid_structural("dynamic call result definition is absent"))?;
    let InstructionKind::Call {
        target: CallTarget::Direct(callee),
        instantiation: Some(instantiation),
        ..
    } = &call.kind
    else {
        return Err(invalid_structural(
            "dynamic structural key is not an authenticated direct call result",
        ));
    };
    let callee = program
        .functions
        .get(callee.index().unwrap_or(usize::MAX))
        .filter(|function| function.id == *callee)
        .ok_or_else(|| invalid_structural("dynamic direct call target is absent"))?;
    let SsaType::TypeParameter(parameter) = callee.signature.result.as_ref() else {
        return Err(invalid_structural("dynamic direct call result is not a type parameter"));
    };
    let binding = instantiation
        .memory_witnesses
        .iter()
        .find(|binding| binding.parameter == *parameter)
        .ok_or_else(|| invalid_structural("dynamic direct call result witness is absent"))?;
    let locator = program
        .memory
        .witnesses
        .binary_search_by_key(&binding.witness.bytes(), |item| item.id.bytes())
        .ok()
        .and_then(|index| u16::try_from(index).ok())
        .ok_or_else(|| invalid_structural("dynamic call result witness exceeds u16"))?;
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
    value: ValueId,
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
    let (operation, key) = match instruction.kind {
        InstructionKind::MemoryWitnessIndependentOwner { .. } => (
            lkjscript_native::StructuralOperation::WitnessIndependentOwner,
            observe_value(function, value, block, locals, builder)?,
        ),
        InstructionKind::MemoryWitnessDispose { .. } => (
            lkjscript_native::StructuralOperation::WitnessDispose,
            read_value(builder, block, locals, value, function.id)?,
        ),
        _ => return Err(invalid_structural("non-witness instruction reached witness lowering")),
    };
    let descriptor = lkjscript_native::StructuralCallDescriptor::new(operation)
        .map_err(LoweringError::backend)?;
    builder
        .structural_call(block, descriptor, vec![locator, key])
        .map_err(|error| {
            LoweringError::new(
                LoweringFailureCode::Backend,
                Some(function.id),
                format!("dynamic witness operation: {error}"),
            )
        })
}
