use super::*;

type NativeResult = Result<lkjscript_native::ValueId, lkjscript_native::PlanError>;

pub(super) fn lower_move(
    function: &Function,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let input = read(builder, block, locals, value, function.id)?;
    if value_type(value_types, value).ok() == Some(ValueType::Unique(UniqueType::ByteVector)) {
        builder.runtime_call(block, RuntimeCallSlot::ByteVectorMove, vec![input])
    } else {
        Ok(input)
    }
}

pub(super) fn lower_end_borrow(
    function: &Function,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let input = read(builder, block, locals, value, function.id)?;
    let slot = match value_type(value_types, value).ok() {
        Some(ValueType::Loan(LoanType::ByteSlice)) => RuntimeCallSlot::ByteSliceEnd,
        Some(ValueType::Loan(LoanType::ByteSliceMut)) => RuntimeCallSlot::ByteSliceMutEnd,
        _ => return Err(lkjscript_native::PlanError::UnknownValue),
    };
    builder.runtime_call(block, slot, vec![input])
}

pub(super) fn lower_drop(
    function: &Function,
    value: ValueId,
    glue: lkjscript_ir::DropGlueIdentity,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    builder: &mut FunctionBuilder,
) -> NativeResult {
    if glue != lkjscript_ir::DropGlueIdentity::ByteVector {
        return Err(lkjscript_native::PlanError::UnknownValue);
    }
    let input = read(builder, block, locals, value, function.id)?;
    builder.runtime_call(block, RuntimeCallSlot::ByteVectorDrop, vec![input])
}

pub(super) fn lower_borrow(
    function: &Function,
    kind: lkjscript_ir::BorrowKind,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let owner = read(builder, block, locals, value, function.id)?;
    let slot = match kind {
        lkjscript_ir::BorrowKind::Shared => RuntimeCallSlot::ByteVectorBorrowShared,
        lkjscript_ir::BorrowKind::Mutable => RuntimeCallSlot::ByteVectorBorrowExclusive,
    };
    builder.runtime_call(block, slot, vec![owner])
}

fn read(
    builder: &mut FunctionBuilder,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value: ValueId,
    function: FunctionId,
) -> NativeResult {
    read_value(builder, block, locals, value, function)
        .map_err(|_| lkjscript_native::PlanError::UnknownValue)
}
