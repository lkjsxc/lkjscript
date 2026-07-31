use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_byte_data(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<&EvalValue>,
        result_type: &crate::SsaType,
    ) -> std::result::Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match operation {
            Op::ByteVectorNew => unary(&arguments, |size| {
                let size = usize::try_from(as_i64(size)?)
                    .map_err(|_| Flow::Trap("new-byte-vector size out of range".into()))?;
                if size > self.config.max_byte_storage_bytes || size > 1_000_000 {
                    return Err(Flow::Trap("new-byte-vector size out of range".into()));
                }
                self.unique.allocate(size)
            }),
            Op::ByteSliceLength => {
                unary(&arguments, |view| self.unique.len(view).map(EvalValue::I64))
            }
            Op::ByteSliceByteAt => binary(&arguments, |view, index| {
                let index = index_value(index, "byte-slice-byte-at")?;
                self.unique.byte_at(view, index).map(EvalValue::I64)
            }),
            Op::ByteSliceMutSetByte => ternary(&arguments, |view, index, byte| {
                let index = index_value(index, "byte-slice-mut-set-byte")?;
                let byte = u8::try_from(as_i64(byte)?)
                    .map_err(|_| Flow::Trap("byte-slice-mut byte out of range".into()))?;
                self.unique.set_byte(view, index, byte)?;
                Ok(EvalValue::Unit)
            }),
            Op::ByteSliceReadU32Le => binary(&arguments, |view, index| {
                let index = index_value(index, "byte-slice-read-u32-little-endian")?;
                self.unique
                    .read_u32_little_endian(view, index)
                    .map(EvalValue::I64)
            }),
            Op::ByteSliceMutWriteU32Le => ternary(&arguments, |view, index, word| {
                let index = index_value(index, "byte-slice-mut-write-u32-little-endian")?;
                let word = u32::try_from(as_i64(word)?).map_err(|_| {
                    Flow::Trap("byte-slice-mut-write-u32-little-endian value out of range".into())
                })?;
                self.unique.write_u32_little_endian(view, index, word)?;
                Ok(EvalValue::Unit)
            }),
            Op::ConvertStringToBytes => unary(&arguments, |text| {
                let bytes = self.string_bytes_copy(text)?;
                self.unique.allocate_bytes(bytes)
            }),
            Op::ConvertBytesToString => unary(&arguments, |value| {
                let bytes = match value {
                    EvalValue::StaticBytes(id) => self
                        .static_bytes
                        .get(*id as usize)
                        .ok_or_else(|| Flow::Trap("stale static bytes index".into()))?
                        .to_vec(),
                    _ => self.unique.copy_bytes(value)?,
                };
                self.utf8_result(&bytes, result_type)
            }),
            _ => unreachable!("runtime operation dispatched to the wrong byte-data family"),
        }
    }

    fn utf8_result(
        &mut self,
        bytes: &[u8],
        result_type: &crate::SsaType,
    ) -> std::result::Result<EvalValue, Flow> {
        match crate::utf8_contract::validate_utf8(bytes) {
            Ok(text) => {
                let payload = self.allocate_string(text.to_owned())?;
                self.allocate_result(result_type, payload, true)
            }
            Err(failure) => {
                let offset = i64::try_from(failure.offset)
                    .map_err(|_| Flow::Trap("UTF-8 offset out of range".into()))?;
                let (_, fields, _) = enum_variant(
                    self.program.program(),
                    result_type,
                    crate::VariantId::new(crate::prelude_contract::RESULT_ERR_ID),
                )
                .map_err(Flow::Trap)?;
                let error_type = fields
                    .first()
                    .ok_or_else(|| Flow::Trap("UTF-8 result error metadata missing".into()))?;
                let error = self.allocate_enum(
                    error_type,
                    failure.kind.variant_id(),
                    vec![EvalValue::I64(offset)],
                )?;
                self.allocate_result(result_type, error, false)
            }
        }
    }
}
