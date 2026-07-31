use super::*;

impl Evaluator<'_> {
    pub(crate) fn consume_fuel(&mut self) -> std::result::Result<(), Flow> {
        if self.fuel == 0 {
            return Err(Flow::Resource("fuel".into()));
        }
        self.fuel -= 1;
        Ok(())
    }

    pub(crate) fn allocate(&mut self) -> std::result::Result<(), Flow> {
        self.allocate_dynamic(0)
    }

    pub(crate) fn charge_aggregate(&mut self) -> std::result::Result<(), Flow> {
        if self.logical_aggregate_constructions >= self.config.max_logical_aggregate_constructions {
            return Err(Flow::Resource("logical_aggregate_constructions".into()));
        }
        self.logical_aggregate_constructions += 1;
        Ok(())
    }

    pub(crate) fn allocate_dynamic(
        &mut self,
        dynamic_bytes: usize,
    ) -> std::result::Result<(), Flow> {
        if self.allocations >= self.config.max_allocations {
            return Err(Flow::Resource("allocations".into()));
        }
        let object_bytes = evaluator_heap_object_bytes().saturating_add(dynamic_bytes);
        let next_heap_bytes = self
            .heap_bytes
            .checked_add(object_bytes)
            .ok_or_else(|| Flow::Resource("heap bytes".into()))?;
        if next_heap_bytes > self.config.max_heap_bytes {
            return Err(Flow::Resource("heap bytes".into()));
        }
        self.allocations += 1;
        self.heap_bytes = next_heap_bytes;
        Ok(())
    }

    pub(crate) fn allocate_path(&mut self, bytes: &[u8]) -> std::result::Result<EvalValue, Flow> {
        let mut copy = Vec::new();
        copy.try_reserve_exact(bytes.len())
            .map_err(|_| Flow::Resource("heap bytes".into()))?;
        copy.extend_from_slice(bytes);
        if structural_eligible(self.program.program(), &crate::SsaType::Path) {
            self.structural_path(copy)
        } else {
            self.allocate_dynamic(copy.capacity())?;
            self.unique.allocate_path(copy)
        }
    }

    pub(crate) fn allocate_string(&mut self, text: String) -> std::result::Result<EvalValue, Flow> {
        if structural_eligible(self.program.program(), &crate::SsaType::Str) {
            self.structural_string(text)
        } else {
            self.allocate_dynamic(text.capacity())?;
            Ok(EvalValue::Str(text))
        }
    }

    pub(crate) fn allocate_enum(
        &mut self,
        ty: &crate::SsaType,
        variant_id: [u8; 32],
        payload: Vec<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        self.construct_enum(ty, crate::VariantId::new(variant_id), payload)
    }

    pub(crate) fn allocate_result(
        &mut self,
        ty: &crate::SsaType,
        payload: EvalValue,
        is_ok: bool,
    ) -> std::result::Result<EvalValue, Flow> {
        let variant = if is_ok {
            crate::prelude_contract::RESULT_OK_ID
        } else {
            crate::prelude_contract::RESULT_ERR_ID
        };
        self.allocate_enum(ty, variant, vec![payload])
    }

    pub(crate) fn allocate_option(
        &mut self,
        ty: &crate::SsaType,
        payload: Option<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        let (variant, payload) = match payload {
            Some(value) => (crate::prelude_contract::OPTION_SOME_ID, vec![value]),
            None => (crate::prelude_contract::OPTION_NONE_ID, Vec::new()),
        };
        self.allocate_enum(ty, variant, payload)
    }

    pub(crate) fn allocate_system_error(
        &mut self,
        result_ty: &crate::SsaType,
        variant: [u8; 32],
        message: &str,
    ) -> std::result::Result<EvalValue, Flow> {
        let (_, result_fields, _) = enum_variant(
            self.program.program(),
            result_ty,
            crate::VariantId::new(crate::prelude_contract::RESULT_ERR_ID),
        )
        .map_err(Flow::Trap)?;
        let error_ty = result_fields
            .first()
            .ok_or_else(|| Flow::Trap("result error payload metadata missing".into()))?;
        let (_, error_fields, _) = enum_variant(
            self.program.program(),
            error_ty,
            crate::VariantId::new(variant),
        )
        .map_err(Flow::Trap)?;
        let code_ty = error_fields
            .first()
            .ok_or_else(|| Flow::Trap("system error code metadata missing".into()))?;
        let detail_ty = error_fields
            .get(1)
            .ok_or_else(|| Flow::Trap("system error detail metadata missing".into()))?;
        let code = self.allocate_option(code_ty, None)?;
        let detail = self.allocate_string(message.to_owned())?;
        let detail = self.allocate_option(detail_ty, Some(detail))?;
        let error = self.allocate_enum(error_ty, variant, vec![code, detail])?;
        self.allocate_result(result_ty, error, false)
    }
}
