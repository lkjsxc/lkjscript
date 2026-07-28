use super::*;

impl Evaluator<'_> {
    pub(super) fn runtime_bytes(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<EvalValue>,
    ) -> Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match (operation, arguments.as_slice()) {
            (Op::BytesLength, [value]) => {
                let len = self.bytes_len(value)?;
                Ok(EvalValue::I64(i64::try_from(len).map_err(|_| {
                    Flow::Trap("bytes-length result exceeds i64".into())
                })?))
            }
            (Op::BytesByteAt, [value, EvalValue::I64(index)]) => {
                let index = index_arg("bytes-byte-at", "index", *index)?;
                let byte = match value {
                    EvalValue::StaticBytes(id) => {
                        self.static_value(*id)?.get(index).copied().ok_or_else(|| {
                            Flow::Trap(format!("bytes-byte-at index {index} out of range"))
                        })?
                    }
                    EvalValue::Bytes(_) | EvalValue::BytesBorrow(_) => {
                        self.unique.bytes_at(value, index)?
                    }
                    _ => return Err(Flow::Trap("bytes-byte-at expects bytes".into())),
                };
                Ok(EvalValue::I64(i64::from(byte)))
            }
            (Op::CloneBytes, [value]) => match value {
                EvalValue::StaticBytes(id) => {
                    let (table, unique) = (&self.static_bytes, &mut self.unique);
                    let bytes = table
                        .get(*id as usize)
                        .ok_or_else(|| Flow::Trap("stale static bytes index".into()))?;
                    unique.clone_static(bytes)
                }
                EvalValue::Bytes(_) | EvalValue::BytesBorrow(_) => self.unique.clone_bytes(value),
                _ => Err(Flow::Trap("clone-bytes expects bytes".into())),
            },
            (Op::CopyBytesSlice, [value, EvalValue::I64(start), EvalValue::I64(len)]) => {
                let start = index_arg("copy-bytes-slice", "start", *start)?;
                let len = index_arg("copy-bytes-slice", "length", *len)?;
                match value {
                    EvalValue::StaticBytes(id) => {
                        let (table, unique) = (&self.static_bytes, &mut self.unique);
                        let bytes = table
                            .get(*id as usize)
                            .ok_or_else(|| Flow::Trap("stale static bytes index".into()))?;
                        unique.copy_static_range(bytes, start, len)
                    }
                    EvalValue::Bytes(_) | EvalValue::BytesBorrow(_) => {
                        self.unique.copy_bytes_range(value, start, len)
                    }
                    _ => Err(Flow::Trap("copy-bytes-slice expects bytes".into())),
                }
            }
            (Op::FreezeByteVector, [value]) => self.unique.freeze(value),
            (Op::ThawBytes, [value]) => match value {
                EvalValue::StaticBytes(id) => {
                    let (table, unique) = (&self.static_bytes, &mut self.unique);
                    let bytes = table
                        .get(*id as usize)
                        .ok_or_else(|| Flow::Trap("stale static bytes index".into()))?;
                    unique.thaw_static(bytes)
                }
                EvalValue::Bytes(_) => self.unique.thaw_dynamic(value),
                _ => Err(Flow::Trap("thaw-bytes expects bytes".into())),
            },
            _ => Err(Flow::Trap(format!(
                "invalid immutable-bytes runtime arguments for {operation:?}"
            ))),
        }
    }

    fn bytes_len(&mut self, value: &EvalValue) -> Result<usize, Flow> {
        match value {
            EvalValue::StaticBytes(id) => Ok(self.static_value(*id)?.len()),
            EvalValue::Bytes(_) | EvalValue::BytesBorrow(_) => self.unique.bytes_length(value),
            _ => Err(Flow::Trap("bytes-length expects bytes".into())),
        }
    }

    fn static_value(&self, id: u32) -> Result<&[u8], Flow> {
        self.static_bytes
            .get(id as usize)
            .map(AsRef::as_ref)
            .ok_or_else(|| Flow::Trap("stale static bytes index".into()))
    }
}

fn index_arg(operation: &str, field: &str, value: i64) -> Result<usize, Flow> {
    usize::try_from(value).map_err(|_| {
        Flow::Trap(format!(
            "{operation} {field} {value} is negative or exceeds the platform range"
        ))
    })
}
