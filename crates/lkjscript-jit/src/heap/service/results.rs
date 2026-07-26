use super::*;

impl JitHeapServices<'_> {
    pub(crate) fn result_value(
        &mut self,
        payload: Value,
        is_ok: bool,
        result_type: ValueType,
    ) -> Result<NativeValue, NativeServiceError> {
        let reference_type = match result_type.reference_type() {
            Some(reference @ ReferenceType::Enum(_, layout))
                if layout == lkjscript_core::RESULT_LAYOUT =>
            {
                reference
            }
            _ => return Err(NativeServiceError::HostFailure),
        };
        let result = self.enum_value(
            lkjscript_core::RESULT_LAYOUT,
            if is_ok { 0 } else { 1 },
            vec![payload],
            reference_type,
        )?;
        self.native_from_value(result, result_type)
    }

    pub(crate) fn result_error(
        &mut self,
        message: &str,
        result_type: ValueType,
        error_type: ReferenceType,
        code_option_type: ReferenceType,
        detail_option_type: ReferenceType,
    ) -> Result<NativeValue, NativeServiceError> {
        let code = self.enum_value(
            lkjscript_core::OPTION_LAYOUT,
            1,
            Vec::new(),
            code_option_type,
        )?;
        let detail = self.allocate(HeapObj::Str(message.into()), ReferenceType::Str)?;
        let detail = self.enum_value(
            lkjscript_core::OPTION_LAYOUT,
            0,
            vec![detail],
            detail_option_type,
        )?;
        let system = self.enum_value(
            lkjscript_core::SYSTEM_ERROR_LAYOUT,
            lkjscript_core::SystemErrorKind::Unsupported.physical_tag(),
            vec![code, detail],
            error_type,
        )?;
        self.result_value(system, false, result_type)
    }
}
