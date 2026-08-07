pub(in crate::run) fn value_from_runtime(
    vm: &Vm<'_>,
    expected: &HostValueType,
    value: Value,
) -> Result<HostValue> {
    let converted = match expected {
        HostValueType::Unit if value.is_unit() => HostValue::Unit,
        HostValueType::Bool => value
            .as_bool()
            .map(HostValue::Bool)
            .ok_or_else(|| Error::msg("host result is not Bool"))?,
        HostValueType::I64 => value
            .as_i64()
            .map(HostValue::I64)
            .ok_or_else(|| Error::msg("host result is not I64"))?,
        HostValueType::F64 => value
            .as_f64_bits()
            .map(HostValue::F64Bits)
            .ok_or_else(|| Error::msg("host result is not F64"))?,
        HostValueType::Resource(kind) => {
            let actual = vm
                .resources
                .owned_kind(value, "aggregate result publication")?;
            if actual != *kind {
                return Err(Error::msg("host result resource kind mismatch"));
            }
            HostValue::Resource { kind: *kind, value }
        }
        _ => {
            return Err(Error::msg(
                "host result requires an explicit key-free structural payload",
            ))
        }
    };
    Ok(converted)
}
