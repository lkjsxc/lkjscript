use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_paths(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        match operation {
            RuntimeOp::PathFromStr => unary(&arguments, |value| {
                self.path_result(as_str(value)?.as_bytes())
            }),
            RuntimeOp::PathFromBuf => unary(&arguments, |value| {
                let buffer = as_buffer(value)?;
                let bytes = buffer.bytes.borrow();
                self.path_result(&bytes)
            }),
            RuntimeOp::PathToBuf => unary(&arguments, |value| {
                let EvalValue::Path(bytes) = value else {
                    return Err(Flow::Trap("expected Path".into()));
                };
                self.allocate_buffer_copy(bytes)
            }),
            RuntimeOp::PathToStr => unary(&arguments, |value| {
                let EvalValue::Path(bytes) = value else {
                    return Err(Flow::Trap("expected Path".into()));
                };
                match crate::utf8_contract::validate_utf8(bytes) {
                    Ok(text) => {
                        let payload = self.allocate_string(text.to_owned())?;
                        self.allocate_result(payload, true)
                    }
                    Err(failure) => {
                        let offset = i64::try_from(failure.offset)
                            .map_err(|_| Flow::Trap("UTF-8 offset out of range".into()))?;
                        let error = self.allocate_enum(
                            crate::prelude_contract::UTF8_ERROR_ID,
                            failure.kind.variant_id(),
                            vec![EvalValue::I64(offset)],
                        )?;
                        self.allocate_result(error, false)
                    }
                }
            }),
            _ => unreachable!("runtime operation dispatched to wrong path family"),
        }
    }

    fn path_result(&mut self, bytes: &[u8]) -> std::result::Result<EvalValue, Flow> {
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
            return self.allocate_system_error(crate::prelude_contract::SYSTEM_IO_ID, detail);
        }
        let path = self.allocate_path(bytes)?;
        self.allocate_result(path, true)
    }
}
