pub(super) fn lower_borrow(
    function: &Function,
    kind: lkjscript_ir::BorrowKind,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let root = structural_type(layouts.structural(), source_type(function, value)?)?;
    let projection = layouts.structural().view(
        root,
        root,
        Vec::new(),
        lkjscript_native::StructuralProjectionKind::Field,
        kind == lkjscript_ir::BorrowKind::Mutable,
    );
    let input = observe_value(function, value, block, locals, builder)?;
    structural_call(
        builder,
        block,
        lkjscript_native::StructuralOperation::Borrow { projection },
        vec![input],
    )
}

pub(in crate::lower) fn copy_call_argument(
    function: &Function,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let ValueType::StructuralOwner(value_type) = value_type(value_types, value)? else {
        return Err(invalid_structural(
            "structural call copy source is not an owner",
        ));
    };
    let input = observe_value(function, value, block, locals, builder)?;
    structural_call(
        builder,
        block,
        lkjscript_native::StructuralOperation::Copy(value_type),
        vec![input],
    )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::lower) fn lower_terminal_cleanup(
    function: &Function,
    cleanup: Option<lkjscript_ir::FailureCleanupRoots>,
    retained: Option<ValueId>,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
) -> Result<(), LoweringError> {
    let Some(cleanup) = cleanup else {
        return Ok(());
    };
    if cleanup.ids().any(|root| {
        root.index()
            .is_none_or(|index| index >= function.failure_cleanups.len())
    }) {
        return Err(invalid_structural(
            "terminal structural cleanup chain is missing",
        ));
    }
    for action in function.failure_cleanup_actions(Some(cleanup)) {
        match action {
            lkjscript_ir::FailureCleanupAction::EndBorrow { value, .. }
                if matches!(
                    value_type(value_types, *value)?,
                    ValueType::StructuralView(_)
                ) =>
            {
                let _ = lower_end_view(function, *value, block, locals, value_types, builder)?;
            }
            lkjscript_ir::FailureCleanupAction::DropOwner {
                value,
                glue: lkjscript_ir::DropGlueIdentity::Structural(_),
                ..
            } if Some(*value) != retained => match value_type(value_types, *value)? {
                ValueType::StructuralOwner(_) => {
                    let _ = lower_drop(function, *value, block, locals, value_types, builder)?;
                }
                ValueType::StructuralDestination(_) => {
                    let (aggregate, storage, initialized) =
                        layouts.structural().destination(function, *value)?;
                    let input = read_value(builder, block, locals, *value, function.id)?;
                    let _ = structural_call(
                        builder,
                        block,
                        lkjscript_native::StructuralOperation::DestinationAbort {
                            aggregate,
                            storage,
                            initialized,
                        },
                        vec![input],
                    )?;
                }
                _ => {
                    return Err(invalid_structural(
                        "terminal structural owner type is invalid",
                    ))
                }
            },
            _ => {}
        }
    }
    Ok(())
}
