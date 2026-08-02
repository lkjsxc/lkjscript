use crate::*;

impl JitIslandServices {
    pub(super) fn retain_list_value(
        &mut self,
        value: NativeValue,
        expected: ValueType,
    ) -> Result<(Value, bool), NativeServiceError> {
        if value.value_type() != expected {
            return Err(NativeServiceError::Trap);
        }
        match value {
            NativeValue::StructuralOwner(owner) => self
                .structural
                .retain_list_owner(owner)
                .map(|value| (value, true)),
            NativeValue::Reference(reference)
                if matches!(reference.reference_type(), ReferenceType::List(_, _, _, _)) =>
            {
                let key = self.list_key(reference)?;
                Ok((
                    if key.is_empty() {
                        Value::EMPTY_LIST
                    } else {
                        Value::from_segmented_list(key.to_word())
                    },
                    false,
                ))
            }
            NativeValue::Unit => Ok((Value::UNIT, false)),
            NativeValue::Bool(value) => Ok((Value::from_bool(value), false)),
            NativeValue::I64(value) => Ok((Value::from_i64(value), false)),
            NativeValue::F64Bits(value) => Ok((Value::from_f64_bits(value), false)),
            _ => Err(NativeServiceError::Trap),
        }
    }

    pub(super) fn native_from_list_value(
        &mut self,
        value: Value,
        expected: ValueType,
    ) -> Result<NativeValue, NativeServiceError> {
        if value.as_structural_root().is_some() {
            return self.structural.clone_list_owner(value, expected);
        }
        match expected {
            ValueType::Reference(reference_type)
                if matches!(reference_type, ReferenceType::List(_, _, _, _)) =>
            {
                let word = if value.is_empty_list() {
                    0
                } else {
                    value.as_segmented_list().ok_or(NativeServiceError::Trap)?
                };
                let key = self
                    .lists
                    .key_from_word(word)
                    .map_err(Self::map_list_error)?;
                self.lists
                    .validate_type(key, reference_layout_key(reference_type))
                    .map_err(Self::map_list_error)?;
                Ok(NativeValue::Reference(
                    lkjscript_native::NativeReference::new(reference_type, word),
                ))
            }
            ValueType::Unit if value.is_unit() => Ok(NativeValue::Unit),
            ValueType::Bool => value
                .as_bool()
                .map(NativeValue::Bool)
                .ok_or(NativeServiceError::Trap),
            ValueType::I64 => value
                .as_i64()
                .map(NativeValue::I64)
                .ok_or(NativeServiceError::Trap),
            ValueType::F64 => value
                .as_f64_bits()
                .map(NativeValue::F64Bits)
                .ok_or(NativeServiceError::Trap),
            _ => Err(NativeServiceError::Trap),
        }
    }
}
