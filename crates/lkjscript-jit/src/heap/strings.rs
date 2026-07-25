use super::*;
use crate::*;

impl JitHeapServices<'_> {
    pub(crate) fn execute_strings(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        let descriptor = site.descriptor();
        let result_type = descriptor.result_type();
        let argument = |index: usize| {
            arguments
                .get(index)
                .copied()
                .ok_or(NativeServiceError::HostFailure)
        };
        let as_i64 = |value: NativeValue| match value {
            NativeValue::I64(value) => Ok(value),
            _ => Err(NativeServiceError::HostFailure),
        };
        let as_f64 = |value: NativeValue| match value {
            NativeValue::F64Bits(bits) => Ok(f64::from_bits(bits)),
            _ => Err(NativeServiceError::HostFailure),
        };
        let as_reference = |value: NativeValue| match value {
            NativeValue::Reference(reference) => Ok(reference),
            _ => Err(NativeServiceError::HostFailure),
        };
        match descriptor.operation() {
            HeapOperation::StrLen | HeapOperation::StrRef => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let text = match self.heap.get(value) {
                    Ok(HeapObj::Str(text)) => text,
                    _ => return self.trap("expected string"),
                };
                if matches!(descriptor.operation(), HeapOperation::StrLen) {
                    Ok(NativeValue::I64(
                        i64::try_from(text.len()).map_err(|_| NativeServiceError::Trap)?,
                    ))
                } else {
                    let index = index(as_i64(argument(1)?)?, "str-ref").map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                    let byte = text.as_bytes().get(index).copied().ok_or_else(|| {
                        self.last_trap = Some("str-ref out of bounds".into());
                        NativeServiceError::Trap
                    })?;
                    Ok(NativeValue::I64(i64::from(byte)))
                }
            }
            HeapOperation::StrAppend | HeapOperation::StrSlice => {
                let first = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let mut text = match self.heap.get(first) {
                    Ok(HeapObj::Str(text)) => text.clone(),
                    _ => return self.trap("expected string"),
                };
                if matches!(descriptor.operation(), HeapOperation::StrAppend) {
                    let second = native_reference_value(self.heap, as_reference(argument(1)?)?)
                        .map_err(|message| {
                            self.last_trap = Some(message);
                            NativeServiceError::Trap
                        })?;
                    let right = match self.heap.get(second) {
                        Ok(HeapObj::Str(text)) => text,
                        _ => return self.trap("expected string"),
                    };
                    text.push_str(right);
                } else {
                    let start = usize::try_from(as_i64(argument(1)?)?).map_err(|_| {
                        self.last_trap = Some("str-slice start out of range".into());
                        NativeServiceError::Trap
                    })?;
                    let end = usize::try_from(as_i64(argument(2)?)?).map_err(|_| {
                        self.last_trap = Some("str-slice end out of range".into());
                        NativeServiceError::Trap
                    })?;
                    let bytes = text.as_bytes().get(start..end).ok_or_else(|| {
                        self.last_trap = Some("str-slice out of bounds".into());
                        NativeServiceError::Trap
                    })?;
                    text = std::str::from_utf8(bytes)
                        .map_err(|_| {
                            self.last_trap = Some("str-slice splits UTF-8".into());
                            NativeServiceError::Trap
                        })?
                        .to_owned();
                }
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(HeapObj::Str(text), reference_type)?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::StrFromByte | HeapOperation::StrFromI64 | HeapOperation::StrFromF64 => {
                let text = match descriptor.operation() {
                    HeapOperation::StrFromByte => {
                        let byte = u8::try_from(as_i64(argument(0)?)?).map_err(|_| {
                            self.last_trap = Some("str-from-byte out of range".into());
                            NativeServiceError::Trap
                        })?;
                        String::from(char::from(byte))
                    }
                    HeapOperation::StrFromI64 => as_i64(argument(0)?)?.to_string(),
                    HeapOperation::StrFromF64 => as_f64(argument(0)?)?.to_string(),
                    _ => return Err(NativeServiceError::HostFailure),
                };
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(HeapObj::Str(text), reference_type)?;
                self.native_from_value(value, result_type)
            }
            _ => Err(NativeServiceError::HostFailure),
        }
    }
}
