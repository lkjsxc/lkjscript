#[allow(clippy::too_many_arguments)]
fn lower_field_projection(
    function: &Function,
    instruction: &Instruction,
    field: u16,
    value: ValueId,
    root: lkjscript_native::StructuralTypeIdentity,
    allow_view_result: bool,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    catalog: &StructuralCatalog,
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let projected = structural_type(catalog, &instruction.ty)?;
    let projection = catalog.view(
        root,
        projected,
        vec![field],
        lkjscript_native::StructuralProjectionKind::Field,
        false,
    );
    let input = observe_value(function, value, block, locals, builder)?;
    let view = structural_call(
        builder,
        block,
        lkjscript_native::StructuralOperation::Borrow {
            projection: projection.clone(),
        },
        vec![input],
    )?;
    if allow_view_result
        && value_type(value_types, instruction.id)?
            == ValueType::StructuralView(projection.view_type())
    {
        return Ok(view);
    }
    let view_local = builder
        .create_local(ValueType::StructuralView(projection.view_type()))
        .map_err(LoweringError::backend)?;
    builder
        .write_local(block, view_local, view)
        .map_err(LoweringError::backend)?;
    let observed_view = builder
        .observe_local(block, view_local)
        .map_err(LoweringError::backend)?;
    let copied = structural_call(
        builder,
        block,
        lkjscript_native::StructuralOperation::CopyView(projection.view_type()),
        vec![observed_view],
    )?;
    let end_view = lkjscript_native::StructuralCallDescriptor::new(
        lkjscript_native::StructuralOperation::EndView(projection.view_type()),
    )
    .map_err(LoweringError::backend)?;
    builder
        .set_instruction_failure_cleanup(
            copied,
            vec![lkjscript_native::FailureCleanupCall::structural(
                end_view,
                view_local,
            )],
        )
        .map_err(LoweringError::backend)?;
    let consumed_view = builder
        .read_local(block, view_local)
        .map_err(LoweringError::backend)?;
    let _ = structural_call(
        builder,
        block,
        lkjscript_native::StructuralOperation::EndView(projection.view_type()),
        vec![consumed_view],
    )?;
    Ok(copied)
}
