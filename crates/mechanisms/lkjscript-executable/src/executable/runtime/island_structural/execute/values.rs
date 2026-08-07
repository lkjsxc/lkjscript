use super::*;

pub(super) const fn witness_locator(word: u64) -> Result<u64, NativeServiceError> {
    Ok(word)
}

pub(super) fn native_value(
    word: u64,
    value_type: ValueType,
) -> Result<NativeValue, NativeServiceError> {
    match value_type {
        ValueType::Unit if word == 0 => Ok(NativeValue::Unit),
        ValueType::Bool if word <= 1 => Ok(NativeValue::Bool(word == 1)),
        ValueType::I64 => Ok(NativeValue::I64(word as i64)),
        ValueType::F64 => Ok(NativeValue::F64Bits(word)),
        ValueType::StructuralKey if word != 0 => Ok(NativeValue::StructuralKey(word)),
        ValueType::StructuralOwner(value_type) if word != 0 => Ok(NativeValue::StructuralOwner(
            NativeStructuralOwner::new(value_type, word),
        )),
        _ => Err(NativeServiceError::Trap),
    }
}
