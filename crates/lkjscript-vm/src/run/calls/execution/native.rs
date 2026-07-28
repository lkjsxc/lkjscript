fn native_from_value(ty: ValueType, value: Value) -> Result<NativeValue> {
    match ty {
        ValueType::Unit if value.is_unit() => Ok(NativeValue::Unit),
        ValueType::Bool => value
            .as_bool()
            .map(NativeValue::Bool)
            .ok_or_else(|| Error::msg("native boundary expected Bool")),
        ValueType::I64 => value
            .as_i64()
            .map(NativeValue::I64)
            .ok_or_else(|| Error::msg("native boundary expected I64")),
        ValueType::F64 => value
            .as_f64_bits()
            .map(NativeValue::F64Bits)
            .ok_or_else(|| Error::msg("native boundary expected F64")),
        ValueType::Unit => Err(Error::msg("native boundary expected Unit")),
        ValueType::Capability(_) | ValueType::Resource(_) | ValueType::Reference(_) => Err(
            Error::msg("VM/native adapter transfer is not enabled in automatic tiering"),
        ),
    }
}

fn value_from_native(value: NativeValue) -> Result<Value> {
    match value {
        NativeValue::Unit => Ok(Value::UNIT),
        NativeValue::Bool(value) => Ok(Value::from_bool(value)),
        NativeValue::I64(value) => Ok(Value::from_i64(value)),
        NativeValue::F64Bits(bits) => Ok(Value::from_f64_bits(bits)),
        NativeValue::Capability(_) | NativeValue::Resource(_) | NativeValue::Reference(_) => {
            unreachable!("automatic tier returned an ineligible native adapter")
        }
    }
}
