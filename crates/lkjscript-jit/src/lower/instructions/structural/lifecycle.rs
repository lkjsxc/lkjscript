pub(super) fn lower_move(
    function: &Function,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let ValueType::StructuralOwner(value_type) = value_type(value_types, value)? else {
        return Err(invalid_structural("structural move source is not an owner"));
    };
    let input = read_value(builder, block, locals, value, function.id)?;
    structural_call(
        builder,
        block,
        lkjscript_native::StructuralOperation::Move(value_type),
        vec![input],
    )
}

pub(in crate::lower) fn lower_drop(
    function: &Function,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let ValueType::StructuralOwner(value_type) = value_type(value_types, value)? else {
        return Err(invalid_structural("structural drop source is not an owner"));
    };
    let input = read_value(builder, block, locals, value, function.id)?;
    structural_call(
        builder,
        block,
        lkjscript_native::StructuralOperation::Drop(value_type),
        vec![input],
    )
}

pub(super) fn lower_end_view(
    function: &Function,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let ValueType::StructuralView(view) = value_type(value_types, value)? else {
        return Err(invalid_structural(
            "structural EndBorrow source is not a view",
        ));
    };
    let input = read_value(builder, block, locals, value, function.id)?;
    structural_call(
        builder,
        block,
        lkjscript_native::StructuralOperation::EndView(view),
        vec![input],
    )
}
