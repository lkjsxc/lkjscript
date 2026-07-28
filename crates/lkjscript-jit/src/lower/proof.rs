use super::*;

pub(super) fn heap_operation(
    operation: RuntimeOp,
    error_types: &[ReferenceType],
) -> Option<HeapOperation> {
    Some(match operation {
        RuntimeOp::SameObject => HeapOperation::SameObject,
        RuntimeOp::ListEqual => HeapOperation::ListEqual,
        RuntimeOp::Cons => HeapOperation::Cons,
        RuntimeOp::Car => HeapOperation::Car,
        RuntimeOp::Cdr => HeapOperation::Cdr,
        RuntimeOp::IsEmptyList => HeapOperation::IsEmptyList,
        RuntimeOp::EmptyStr => HeapOperation::EmptyStr,
        RuntimeOp::BufNew => HeapOperation::BufNew,
        RuntimeOp::BufLen => HeapOperation::BufLen,
        RuntimeOp::BufRef => HeapOperation::BufRef,
        RuntimeOp::BufSet => HeapOperation::BufSet,
        RuntimeOp::BufClone => HeapOperation::BufClone,
        RuntimeOp::BufFromStr => HeapOperation::BufFromStr,
        RuntimeOp::BufToStr => HeapOperation::BufToStr {
            error_type: *error_types.first()?,
        },
        RuntimeOp::BufSlice => {
            let [error_type, code_option_type, detail_option_type] = error_types else {
                return None;
            };
            HeapOperation::BufSlice {
                error_type: *error_type,
                code_option_type: *code_option_type,
                detail_option_type: *detail_option_type,
            }
        }
        RuntimeOp::BufGetU32 => HeapOperation::BufGetU32,
        RuntimeOp::BufSetU32 => HeapOperation::BufSetU32,
        RuntimeOp::StrLen => HeapOperation::StrLen,
        RuntimeOp::StrRef => HeapOperation::StrRef,
        RuntimeOp::StrAppend => HeapOperation::StrAppend,
        RuntimeOp::StrSlice => HeapOperation::StrSlice,
        RuntimeOp::StrFromByte => HeapOperation::StrFromByte,
        RuntimeOp::StrFromI64 => HeapOperation::StrFromI64,
        RuntimeOp::StrFromF64 => HeapOperation::StrFromF64,
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
        HeapOperation::ConstantStr(_)
            | HeapOperation::EmptyStr
            | HeapOperation::ProductValue { .. }
            | HeapOperation::WithProductField { .. }
            | HeapOperation::EnumValue { .. }
            | HeapOperation::Cons
            | HeapOperation::BufNew
            | HeapOperation::BufClone
            | HeapOperation::BufFromStr
            | HeapOperation::BufToStr { .. }
            | HeapOperation::BufSlice { .. }
            | HeapOperation::StrAppend
            | HeapOperation::StrSlice
            | HeapOperation::StrFromByte
            | HeapOperation::StrFromI64
            | HeapOperation::StrFromF64
            | HeapOperation::F64FromI64Exact { .. }
            | HeapOperation::I64FromF64Exact { .. }
            | HeapOperation::I64FromF64Trunc { .. }
    ) {
        AllocationClass::Bounded
    } else {
        AllocationClass::None
    };
    let store = match operation {
        HeapOperation::BufSet | HeapOperation::BufSetU32 => StoreClass::Scalar,
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
        | ValueType::Capability(_)
        | ValueType::Resource(_)
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
