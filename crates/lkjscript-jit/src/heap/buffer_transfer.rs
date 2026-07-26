use super::*;
use crate::*;

impl JitHeapServices<'_> {
    pub(crate) fn execute_buffer_transfer(
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
        let _as_f64 = |value: NativeValue| match value {
            NativeValue::F64Bits(bits) => Ok(f64::from_bits(bits)),
            _ => Err(NativeServiceError::HostFailure),
        };
        let as_reference = |value: NativeValue| match value {
            NativeValue::Reference(reference) => Ok(reference),
            _ => Err(NativeServiceError::HostFailure),
        };
        match descriptor.operation() {
            HeapOperation::BufClone | HeapOperation::BufFromStr => {
                let bytes = if matches!(descriptor.operation(), HeapOperation::BufClone) {
                    let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                        .map_err(|message| {
                            self.last_trap = Some(message);
                            NativeServiceError::Trap
                        })?;
                    match self.heap.get(value) {
                        Ok(HeapObj::Buf(bytes)) => bytes.clone(),
                        _ => return self.trap("expected buf"),
                    }
                } else {
                    let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                        .map_err(|message| {
                            self.last_trap = Some(message);
                            NativeServiceError::Trap
                        })?;
                    match self.heap.get(value) {
                        Ok(HeapObj::Str(text)) => text.as_bytes().to_vec(),
                        _ => return self.trap("expected string"),
                    }
                };
                if bytes.len() > MAX_BUFFER_BYTES {
                    return self.trap("buf-from-str string exceeds buffer limit");
                }
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(HeapObj::Buf(bytes), reference_type)?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::BufToStr { .. } | HeapOperation::BufSlice { .. } => {
                let buffer = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let bytes = match self.heap.get(buffer) {
                    Ok(HeapObj::Buf(bytes)) => bytes.clone(),
                    _ => return self.trap("expected buf"),
                };
                match descriptor.operation() {
                    HeapOperation::BufToStr { error_type } => {
                        match lkjscript_core::validate_utf8(&bytes) {
                            Ok(text) => {
                                let payload =
                                    self.allocate(HeapObj::Str(text.into()), ReferenceType::Str)?;
                                self.result_value(payload, true, result_type)
                            }
                            Err(error) => {
                                let offset = i64::try_from(error.offset)
                                    .map_err(|_| NativeServiceError::HostFailure)?;
                                let offset = self.value_from_native(NativeValue::I64(offset))?;
                                let payload = self.enum_value(
                                    lkjscript_core::UTF8_ERROR_LAYOUT,
                                    error.kind.physical_tag(),
                                    vec![offset],
                                    *error_type,
                                )?;
                                self.result_value(payload, false, result_type)
                            }
                        }
                    }
                    HeapOperation::BufSlice {
                        error_type,
                        code_option_type,
                        detail_option_type,
                    } => {
                        let offset = match usize::try_from(as_i64(argument(1)?)?) {
                            Ok(offset) => offset,
                            Err(_) => {
                                return self.result_error(
                                    "buf-slice offset out of range",
                                    result_type,
                                    *error_type,
                                    *code_option_type,
                                    *detail_option_type,
                                )
                            }
                        };
                        let length = match usize::try_from(as_i64(argument(2)?)?) {
                            Ok(length) => length,
                            Err(_) => {
                                return self.result_error(
                                    "buf-slice length out of range",
                                    result_type,
                                    *error_type,
                                    *code_option_type,
                                    *detail_option_type,
                                )
                            }
                        };
                        let Some(end) = offset.checked_add(length) else {
                            return self.result_error(
                                "buf-slice range overflow",
                                result_type,
                                *error_type,
                                *code_option_type,
                                *detail_option_type,
                            );
                        };
                        let Some(slice) = bytes.get(offset..end) else {
                            return self.result_error(
                                "buf-slice range out of bounds",
                                result_type,
                                *error_type,
                                *code_option_type,
                                *detail_option_type,
                            );
                        };
                        let payload =
                            self.allocate(HeapObj::Buf(slice.to_vec()), ReferenceType::Buf)?;
                        self.result_value(payload, true, result_type)
                    }
                    _ => Err(NativeServiceError::HostFailure),
                }
            }
            _ => Err(NativeServiceError::HostFailure),
        }
    }
}
