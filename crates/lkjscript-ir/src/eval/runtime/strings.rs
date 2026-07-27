use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_strings(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match operation {
            Op::StrLen => unary(&arguments, |text| {
                let length = i64::try_from(as_str(text)?.len())
                    .map_err(|_| Flow::Trap("str-len out of range".into()))?;
                Ok(EvalValue::I64(length))
            }),
            Op::StrRef => binary(&arguments, |text, index| {
                let index = index_value(index, "string-byte-at")?;
                let byte = as_str(text)?
                    .as_bytes()
                    .get(index)
                    .copied()
                    .ok_or_else(|| Flow::Trap("str-ref out of bounds".into()))?;
                Ok(EvalValue::I64(i64::from(byte)))
            }),
            Op::StrAppend => binary(&arguments, |left, right| {
                let mut result = as_str(left)?.to_owned();
                result.push_str(as_str(right)?);
                self.allocate()?;
                Ok(EvalValue::Str(result))
            }),
            Op::StrSlice => ternary(&arguments, |text, start, end| {
                let start = index_value(start, "copy-string-byte-slice")?;
                let end = index_value(end, "copy-string-byte-slice")?;
                let bytes = as_str(text)?.as_bytes();
                let slice = bytes
                    .get(start..end)
                    .ok_or_else(|| Flow::Trap("str-slice out of bounds".into()))?;
                let result = std::str::from_utf8(slice)
                    .map_err(|_| Flow::Trap("str-slice splits UTF-8".into()))?;
                self.allocate()?;
                Ok(EvalValue::Str(result.to_owned()))
            }),
            Op::StrFromByte => unary(&arguments, |value| {
                let byte = u8::try_from(as_i64(value)?)
                    .map_err(|_| Flow::Trap("str-from-byte out of range".into()))?;
                self.allocate()?;
                Ok(EvalValue::Str(String::from(char::from(byte))))
            }),
            Op::StrFromI64 => unary(&arguments, |value| {
                self.allocate()?;
                Ok(EvalValue::Str(as_i64(value)?.to_string()))
            }),
            Op::StrFromF64 => unary(&arguments, |value| {
                self.allocate()?;
                Ok(EvalValue::Str(as_f64_exact(value)?.to_string()))
            }),
            _ => unreachable!("runtime operation dispatched to the wrong family"),
        }
    }
}
