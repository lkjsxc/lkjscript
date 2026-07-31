use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_strings(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<&EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match operation {
            Op::StrLen => unary(&arguments, |text| {
                let length = i64::try_from(self.string_bytes_copy(text)?.len())
                    .map_err(|_| Flow::Trap("str-len out of range".into()))?;
                Ok(EvalValue::I64(length))
            }),
            Op::StrRef => binary(&arguments, |text, index| {
                let index = index_value(index, "string-byte-at")?;
                let byte = self
                    .string_bytes_copy(text)?
                    .get(index)
                    .copied()
                    .ok_or_else(|| Flow::Trap("str-ref out of bounds".into()))?;
                Ok(EvalValue::I64(i64::from(byte)))
            }),
            Op::StrAppend => binary(&arguments, |left, right| {
                let left = self.string_bytes_copy(left)?;
                let right = self.string_bytes_copy(right)?;
                let capacity = left
                    .len()
                    .checked_add(right.len())
                    .ok_or_else(|| Flow::Resource("string bytes".into()))?;
                let mut bytes = Vec::new();
                bytes
                    .try_reserve_exact(capacity)
                    .map_err(|_| Flow::Resource("string bytes".into()))?;
                bytes.extend_from_slice(&left);
                bytes.extend_from_slice(&right);
                let text = String::from_utf8(bytes)
                    .map_err(|_| Flow::Trap("invalid structural UTF-8".into()))?;
                self.allocate_string(text)
            }),
            Op::StrSlice => ternary(&arguments, |text, start, end| {
                let start = index_value(start, "copy-string-byte-slice")?;
                let end = index_value(end, "copy-string-byte-slice")?;
                let bytes = self.string_bytes_copy(text)?;
                let slice = bytes
                    .get(start..end)
                    .ok_or_else(|| Flow::Trap("str-slice out of bounds".into()))?;
                let text = std::str::from_utf8(slice)
                    .map_err(|_| Flow::Trap("str-slice splits UTF-8".into()))?;
                self.allocate_string(text.to_owned())
            }),
            Op::StrFromByte => unary(&arguments, |value| {
                let byte = u8::try_from(as_i64(value)?)
                    .map_err(|_| Flow::Trap("str-from-byte out of range".into()))?;
                self.allocate_string(String::from(char::from(byte)))
            }),
            Op::StrFromI64 => unary(&arguments, |value| {
                self.allocate_string(as_i64(value)?.to_string())
            }),
            Op::StrFromF64 => unary(&arguments, |value| {
                self.allocate_string(as_f64_exact(value)?.to_string())
            }),
            _ => unreachable!("runtime operation dispatched to the wrong family"),
        }
    }
}
