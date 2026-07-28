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
    match value_type(value_types, value).ok() {
        Some(ValueType::Unique(UniqueType::ByteVector)) => {
            builder.runtime_call(block, RuntimeCallSlot::ByteVectorMove, vec![input])
        }
        Some(ValueType::Unique(UniqueType::Bytes)) => {
            builder.runtime_call(block, RuntimeCallSlot::BytesMove, vec![input])
        }
        _ => Ok(input),
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
        Some(ValueType::Loan(LoanType::Bytes)) => RuntimeCallSlot::BytesEndBorrow,
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
    let slot = match glue {
        lkjscript_ir::DropGlueIdentity::ByteVector => RuntimeCallSlot::ByteVectorDrop,
        lkjscript_ir::DropGlueIdentity::Bytes => RuntimeCallSlot::BytesDrop,
        _ => return Err(lkjscript_native::PlanError::UnknownValue),
    };
    let input = read(builder, block, locals, value, function.id)?;
    builder.runtime_call(block, slot, vec![input])
}

pub(super) fn lower_borrow(
    function: &Function,
    kind: lkjscript_ir::BorrowKind,
    value: ValueId,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let owner = read(builder, block, locals, value, function.id)?;
    if value_type(value_types, value).ok() == Some(ValueType::Unique(UniqueType::Bytes)) {
        return match kind {
            lkjscript_ir::BorrowKind::Shared => {
                builder.runtime_call(block, RuntimeCallSlot::BytesBorrowShared, vec![owner])
            }
            lkjscript_ir::BorrowKind::Mutable => Err(lkjscript_native::PlanError::UnknownValue),
        };
    }
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
