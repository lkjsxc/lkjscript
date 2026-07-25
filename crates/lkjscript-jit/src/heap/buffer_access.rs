use super::*;
use crate::*;

impl JitHeapServices<'_> {
    pub(crate) fn execute_buffer_access(
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
            HeapOperation::BufNew => {
                let size = usize::try_from(as_i64(argument(0)?)?).map_err(|_| {
                    self.last_trap = Some("buf-new size out of range".into());
                    NativeServiceError::Trap
                })?;
                if size > MAX_BUFFER_BYTES {
                    return self.trap("buf-new size out of range");
                }
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(HeapObj::Buf(vec![0; size]), reference_type)?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::BufLen | HeapOperation::BufRef | HeapOperation::BufGetU32 => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let HeapObj::Buf(bytes) =
                    self.heap.get(value).map_err(|_| NativeServiceError::Trap)?
                else {
                    return self.trap("expected buf");
                };
                match descriptor.operation() {
                    HeapOperation::BufLen => Ok(NativeValue::I64(
                        i64::try_from(bytes.len()).map_err(|_| NativeServiceError::Trap)?,
                    )),
                    HeapOperation::BufRef => {
                        let index = index(as_i64(argument(1)?)?, "buf-ref").map_err(|message| {
                            self.last_trap = Some(message);
                            NativeServiceError::Trap
                        })?;
                        let byte = bytes.get(index).copied().ok_or_else(|| {
                            self.last_trap = Some("buf-ref out of bounds".into());
                            NativeServiceError::Trap
                        })?;
                        Ok(NativeValue::I64(i64::from(byte)))
                    }
                    HeapOperation::BufGetU32 => {
                        let index =
                            index(as_i64(argument(1)?)?, "buf-get-u32").map_err(|message| {
                                self.last_trap = Some(message);
                                NativeServiceError::Trap
                            })?;
                        let end = index.checked_add(4).ok_or_else(|| {
                            self.last_trap = Some("buf-get-u32 index overflow".into());
                            NativeServiceError::Trap
                        })?;
                        let slice = bytes.get(index..end).ok_or_else(|| {
                            self.last_trap = Some("buf-get-u32 out of bounds".into());
                            NativeServiceError::Trap
                        })?;
                        let mut word = [0; 4];
                        word.copy_from_slice(slice);
                        Ok(NativeValue::I64(i64::from(u32::from_le_bytes(word))))
                    }
                    _ => Err(NativeServiceError::HostFailure),
                }
            }
            HeapOperation::BufSet | HeapOperation::BufSetU32 => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let number = as_i64(argument(2)?)?;
                let operation = if matches!(descriptor.operation(), HeapOperation::BufSet) {
                    "buf-set"
                } else {
                    "buf-set-u32"
                };
                let index = index(as_i64(argument(1)?)?, operation).map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })?;
                if matches!(descriptor.operation(), HeapOperation::BufSet) {
                    let byte = u8::try_from(number).map_err(|_| {
                        self.last_trap = Some("buf-set byte out of range".into());
                        NativeServiceError::Trap
                    })?;
                    match self.heap.get(value) {
                        Ok(HeapObj::Buf(bytes)) if index < bytes.len() => {}
                        Ok(HeapObj::Buf(_)) => return self.trap("buf-set out of bounds"),
                        _ => return self.trap("expected buf"),
                    }
                    self.mutate(value, |object| {
                        let HeapObj::Buf(bytes) = object else {
                            return Err(CoreError::msg("expected buf"));
                        };
                        bytes[index] = byte;
                        Ok(())
                    })?;
                } else {
                    let end = index.checked_add(4).ok_or_else(|| {
                        self.last_trap = Some("buf-set-u32 index overflow".into());
                        NativeServiceError::Trap
                    })?;
                    let number = u32::try_from(number).map_err(|_| {
                        self.last_trap = Some("buf-set-u32 value out of range".into());
                        NativeServiceError::Trap
                    })?;
                    match self.heap.get(value) {
                        Ok(HeapObj::Buf(bytes)) if end <= bytes.len() => {}
                        Ok(HeapObj::Buf(_)) => return self.trap("buf-set-u32 out of bounds"),
                        _ => return self.trap("expected buf"),
                    }
                    self.mutate(value, |object| {
                        let HeapObj::Buf(bytes) = object else {
                            return Err(CoreError::msg("expected buf"));
                        };
                        bytes[index..end].copy_from_slice(&number.to_le_bytes());
                        Ok(())
                    })?;
                }
                Ok(NativeValue::Unit)
            }
            _ => Err(NativeServiceError::HostFailure),
        }
    }
}
