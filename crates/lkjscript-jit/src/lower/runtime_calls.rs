use super::*;

pub(super) fn lower_runtime(
    function: &Function,
    operation: RuntimeOp,
    arguments: &[ValueId],
    context: RuntimeLoweringContext<'_>,
    builder: &mut FunctionBuilder,
) -> Result<lkjscript_native::ValueId, lkjscript_native::PlanError> {
    let block = context.block;
    let value_types = context.value_types;
    let values = read_values(builder, block, context.locals, arguments, function.id)
        .map_err(|_| lkjscript_native::PlanError::UnknownValue)?;
    let input_types = arguments
        .iter()
        .map(|argument| value_type(value_types, *argument))
        .collect::<Result<Vec<_>, _>>()?;
    let reference_equality = operation == RuntimeOp::EqualValue
        && input_types
            .first()
            .is_some_and(|ty| matches!(ty, ValueType::Reference(_)));
    let heap = heap_operation(operation);
    if reference_equality || heap.is_some() {
        let operation = if reference_equality {
            HeapOperation::EqualValue
        } else {
            heap.ok_or(lkjscript_native::PlanError::InvalidHeapCall)?
        };
        return builder.heap_call(
            block,
            heap_descriptor(operation, input_types, context.result_type)?,
            values,
        );
    }
    if let Some(result) = lower_bytes_runtime(operation, &input_types, block, &values, builder) {
        return result;
    }
    match operation {
        RuntimeOp::StdinHandle => builder.runtime_call(block, RuntimeCallSlot::StdinHandle, values),
        RuntimeOp::ByteVectorNew => {
            builder.runtime_call(block, RuntimeCallSlot::ByteVectorNew, values)
        }
        RuntimeOp::ByteSliceLength => {
            builder.runtime_call(block, RuntimeCallSlot::ByteSliceLength, values)
        }
        RuntimeOp::ByteSliceByteAt => {
            builder.runtime_call(block, RuntimeCallSlot::ByteSliceByteAt, values)
        }
        RuntimeOp::ByteSliceReadU32Le => {
            builder.runtime_call(block, RuntimeCallSlot::ByteSliceReadU32Le, values)
        }
        RuntimeOp::ByteSliceMutSetByte => {
            builder.runtime_call(block, RuntimeCallSlot::ByteSliceMutSetByte, values)
        }
        RuntimeOp::ByteSliceMutWriteU32Le => {
            builder.runtime_call(block, RuntimeCallSlot::ByteSliceMutWriteU32Le, values)
        }
        RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide => {
            let [left, right] = two_values(&values)?;
            match value_type(value_types, arguments[0])? {
                ValueType::I64 if value_type(value_types, arguments[1])? == ValueType::I64 => {
                    match operation {
                        RuntimeOp::Add => builder.i64_add(block, left, right),
                        RuntimeOp::Subtract => builder.i64_sub(block, left, right),
                        RuntimeOp::Multiply => builder.i64_mul(block, left, right),
                        RuntimeOp::Divide => builder.i64_div(block, left, right),
                        _ => Err(lkjscript_native::PlanError::UnknownValue),
                    }
                }
                _ => {
                    let left = convert_to_f64(
                        builder,
                        block,
                        left,
                        value_type(value_types, arguments[0])?,
                    )?;
                    let right = convert_to_f64(
                        builder,
                        block,
                        right,
                        value_type(value_types, arguments[1])?,
                    )?;
                    match operation {
                        RuntimeOp::Add => builder.f64_add(block, left, right),
                        RuntimeOp::Subtract => builder.f64_sub(block, left, right),
                        RuntimeOp::Multiply => builder.f64_mul(block, left, right),
                        RuntimeOp::Divide => builder.f64_div(block, left, right),
                        _ => Err(lkjscript_native::PlanError::UnknownValue),
                    }
                }
            }
        }
        RuntimeOp::Less | RuntimeOp::LessEqual | RuntimeOp::Greater | RuntimeOp::GreaterEqual => {
            let [left, right] = two_values(&values)?;
            let comparison_i64 = match operation {
                RuntimeOp::Less => I64Comparison::LessThan,
                RuntimeOp::LessEqual => I64Comparison::LessThanOrEqual,
                RuntimeOp::Greater => I64Comparison::GreaterThan,
                RuntimeOp::GreaterEqual => I64Comparison::GreaterThanOrEqual,
                _ => return Err(lkjscript_native::PlanError::UnknownValue),
            };
            if value_type(value_types, arguments[0])? == ValueType::I64
                && value_type(value_types, arguments[1])? == ValueType::I64
            {
                builder.i64_compare(block, comparison_i64, left, right)
            } else {
                let left =
                    convert_to_f64(builder, block, left, value_type(value_types, arguments[0])?)?;
                let right = convert_to_f64(
                    builder,
                    block,
                    right,
                    value_type(value_types, arguments[1])?,
                )?;
                let comparison = match operation {
                    RuntimeOp::Less => F64Comparison::OrderedLessThan,
                    RuntimeOp::LessEqual => F64Comparison::OrderedLessThanOrEqual,
                    RuntimeOp::Greater => F64Comparison::OrderedGreaterThan,
                    RuntimeOp::GreaterEqual => F64Comparison::OrderedGreaterThanOrEqual,
                    _ => return Err(lkjscript_native::PlanError::UnknownValue),
                };
                builder.f64_compare(block, comparison, left, right)
            }
        }
        RuntimeOp::EqualValue => {
            let [left, right] = two_values(&values)?;
            match value_type(value_types, arguments[0])? {
                ValueType::Unit => builder.bool_const(block, true),
                ValueType::Bool => builder.bool_compare(block, BoolComparison::Equal, left, right),
                ValueType::I64 => builder.i64_compare(block, I64Comparison::Equal, left, right),
                ValueType::F64 => {
                    builder.f64_compare(block, F64Comparison::OrderedEqual, left, right)
                }
                ValueType::StaticBytes
                | ValueType::StaticString(_)
                | ValueType::Capability(_)
                | ValueType::Resource(_)
                | ValueType::Unique(_)
                | ValueType::Loan(_)
                | ValueType::StructuralOwner(_)
                | ValueType::StructuralView(_)
                | ValueType::StructuralDestination(_)
                | ValueType::Reference(_) => Err(lkjscript_native::PlanError::UnknownValue),
            }
        }
        RuntimeOp::F64BitsEqual => {
            let [left, right] = two_values(&values)?;
            builder.f64_bits_equal(block, left, right)
        }
        RuntimeOp::Not => builder.bool_not(block, one_value(&values)?),
        RuntimeOp::BitAnd => {
            let [left, right] = two_values(&values)?;
            builder.i64_bit_and(block, left, right)
        }
        RuntimeOp::BitOr => {
            let [left, right] = two_values(&values)?;
            builder.i64_bit_or(block, left, right)
        }
        RuntimeOp::BitXor => {
            let [left, right] = two_values(&values)?;
            builder.i64_bit_xor(block, left, right)
        }
        _ => Err(lkjscript_native::PlanError::UnknownValue),
    }
}
