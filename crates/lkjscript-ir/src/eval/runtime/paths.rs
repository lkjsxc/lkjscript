use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_paths(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<&EvalValue>,
        result_type: &crate::SsaType,
    ) -> std::result::Result<EvalValue, Flow> {
        match operation {
            RuntimeOp::PathFromStr => unary(&arguments, |value| {
                let bytes = self.string_bytes_copy(value)?;
                self.path_result(&bytes, result_type)
            }),
            RuntimeOp::PathFromBytes => unary(&arguments, |value| {
                let bytes = match value {
                    EvalValue::StaticBytes(id) => self
                        .static_bytes
                        .get(*id as usize)
                        .ok_or_else(|| Flow::Trap("stale static bytes index".into()))?
                        .to_vec(),
                    _ => self.unique.copy_bytes(value)?,
                };
                self.path_result(&bytes, result_type)
            }),
            RuntimeOp::PathToBytes => unary(&arguments, |value| {
                let bytes = self.path_bytes_copy(value)?;
                self.unique.allocate_bytes(bytes)
            }),
            RuntimeOp::PathToStr => unary(&arguments, |value| {
                let bytes = self.path_bytes_copy(value)?;
                match crate::utf8_contract::validate_utf8(&bytes) {
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
                        let error_type = fields.first().ok_or_else(|| {
                            Flow::Trap("path UTF-8 result error metadata missing".into())
                        })?;
                        let error = self.allocate_enum(
                            error_type,
                            failure.kind.variant_id(),
                            vec![EvalValue::I64(offset)],
                        )?;
                        self.allocate_result(result_type, error, false)
                    }
                }
            }),
            _ => unreachable!("runtime operation dispatched to wrong path family"),
        }
    }

    fn path_result(
        &mut self,
        bytes: &[u8],
        result_type: &crate::SsaType,
    ) -> std::result::Result<EvalValue, Flow> {
        let detail = if bytes.is_empty() || bytes.len() > 4095 {
            Some("Path must contain 1 through 4095 bytes")
        } else if bytes.first() != Some(&b'/') {
            Some("Path must be absolute")
        } else if bytes.contains(&0) {
            Some("Path contains an interior NUL")
        } else {
            None
        };
        if let Some(detail) = detail {
            return self.allocate_system_error(
                result_type,
                crate::prelude_contract::SYSTEM_IO_ID,
                detail,
            );
        }
        let path = self.allocate_path(bytes)?;
        self.allocate_result(result_type, path, true)
    }
}
