use super::*;

pub(in crate::lower) fn lower_bytes_runtime(
    operation: RuntimeOp,
    input_types: &[ValueType],
    block: lkjscript_native::BlockId,
    values: &[lkjscript_native::ValueId],
    builder: &mut FunctionBuilder,
) -> Option<Result<lkjscript_native::ValueId, lkjscript_native::PlanError>> {
    let slots = match operation {
        RuntimeOp::BytesLength => (
            RuntimeCallSlot::StaticBytesLength,
            RuntimeCallSlot::BytesLength,
        ),
        RuntimeOp::BytesByteAt => (
            RuntimeCallSlot::StaticBytesByteAt,
            RuntimeCallSlot::BytesByteAt,
        ),
        RuntimeOp::CloneBytes => (
            RuntimeCallSlot::StaticBytesClone,
            RuntimeCallSlot::BytesClone,
        ),
        RuntimeOp::CopyBytesSlice => (
            RuntimeCallSlot::StaticBytesCopySlice,
            RuntimeCallSlot::BytesCopySlice,
        ),
        RuntimeOp::ThawBytes => (RuntimeCallSlot::StaticBytesThaw, RuntimeCallSlot::ThawBytes),
        RuntimeOp::FreezeByteVector => {
            return Some(builder.runtime_call(
                block,
                RuntimeCallSlot::FreezeByteVector,
                values.to_vec(),
            ));
        }
        _ => return None,
    };
    Some(
        bytes_runtime_slot(input_types, slots)
            .and_then(|slot| builder.runtime_call(block, slot, values.to_vec())),
    )
}

fn bytes_runtime_slot(
    input_types: &[ValueType],
    (static_slot, dynamic_slot): (RuntimeCallSlot, RuntimeCallSlot),
) -> Result<RuntimeCallSlot, lkjscript_native::PlanError> {
    match input_types.first() {
        Some(ValueType::StaticBytes) => Ok(static_slot),
        Some(ValueType::Unique(UniqueType::Bytes) | ValueType::Loan(LoanType::Bytes)) => {
            Ok(dynamic_slot)
        }
        _ => Err(lkjscript_native::PlanError::UnknownValue),
    }
}
