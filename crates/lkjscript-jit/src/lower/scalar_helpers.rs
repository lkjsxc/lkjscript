use super::*;

pub(super) fn heap_operation(operation: RuntimeOp) -> Option<HeapOperation> {
    Some(match operation {
        RuntimeOp::ListEqual => HeapOperation::ListEqual,
        RuntimeOp::Cons => HeapOperation::Cons,
        RuntimeOp::Car => HeapOperation::Car,
        RuntimeOp::Cdr => HeapOperation::Cdr,
        RuntimeOp::IsEmptyList => HeapOperation::IsEmptyList,
        _ => return None,
    })
}

pub(super) fn heap_descriptor(
    operation: HeapOperation,
    input_types: Vec<ValueType>,
    result_type: ValueType,
) -> Result<HeapCallDescriptor, lkjscript_native::PlanError> {
    let allocation = if matches!(
        operation,
        HeapOperation::ProductValue { .. }
            | HeapOperation::WithProductField { .. }
            | HeapOperation::Cons
    ) {
        AllocationClass::Bounded
    } else {
        AllocationClass::None
    };
    let store = match operation {
        _ if allocation == AllocationClass::Bounded => StoreClass::Initialization,
        _ => StoreClass::None,
    };
    HeapCallDescriptor::new(operation, input_types, result_type, allocation, store)
}

pub(super) fn convert_to_f64(
    builder: &mut FunctionBuilder,
    block: lkjscript_native::BlockId,
    value: lkjscript_native::ValueId,
    ty: ValueType,
) -> Result<lkjscript_native::ValueId, lkjscript_native::PlanError> {
    match ty {
        ValueType::F64 => Ok(value),
        ValueType::I64 => builder.i64_to_f64(block, value),
        ValueType::Bool
        | ValueType::Unit
        | ValueType::StaticBytes
        | ValueType::StaticString(_)
        | ValueType::Capability(_)
        | ValueType::Resource(_)
        | ValueType::Unique(_)
        | ValueType::Loan(_)
        | ValueType::StructuralKey
        | ValueType::MemoryWitnessLocator
        | ValueType::StructuralOwner(_)
        | ValueType::StructuralView(_)
        | ValueType::StructuralDestination(_)
        | ValueType::Reference(_) => Err(lkjscript_native::PlanError::UnknownValue),
    }
}

pub(super) fn two_values(
    values: &[lkjscript_native::ValueId],
) -> Result<[lkjscript_native::ValueId; 2], lkjscript_native::PlanError> {
    match values {
        [left, right] => Ok([*left, *right]),
        _ => Err(lkjscript_native::PlanError::UnknownValue),
    }
}

pub(super) fn one_value(
    values: &[lkjscript_native::ValueId],
) -> Result<lkjscript_native::ValueId, lkjscript_native::PlanError> {
    match values {
        [value] => Ok(*value),
        _ => Err(lkjscript_native::PlanError::UnknownValue),
    }
}

pub(super) fn value_type(
    value_types: &[ValueType],
    value: ValueId,
) -> Result<ValueType, lkjscript_native::PlanError> {
    value
        .index()
        .and_then(|index| value_types.get(index))
        .copied()
        .ok_or(lkjscript_native::PlanError::UnknownValue)
}
