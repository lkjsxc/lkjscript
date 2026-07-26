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

    pub(crate) fn allocate_buffer(
        &mut self,
        bytes: Vec<u8>,
    ) -> std::result::Result<EvalValue, Flow> {
        self.allocate_dynamic(bytes.capacity())?;
        let id = self.next_buffer_id;
        self.next_buffer_id = self.next_buffer_id.saturating_add(1);
        Ok(EvalValue::Buf(EvalBuffer {
            id,
            bytes: Rc::new(RefCell::new(bytes)),
        }))
    }

    pub(crate) fn allocate_string(&mut self, text: String) -> std::result::Result<EvalValue, Flow> {
        self.allocate_dynamic(text.capacity())?;
        Ok(EvalValue::Str(text))
    }

    pub(crate) fn allocate_enum(
        &mut self,
        enum_id: [u8; 32],
        variant_id: [u8; 32],
        payload: Vec<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        let enum_id = crate::EnumId::new(enum_id);
        let variant = crate::VariantId::new(variant_id);
        let definition = self
            .program
            .program()
            .enums
            .iter()
            .find(|item| item.id == enum_id)
            .ok_or_else(|| Flow::Trap("prelude enum metadata missing".into()))?;
        let selected = definition
            .variants
            .iter()
            .find(|item| item.id == variant)
            .ok_or_else(|| Flow::Trap("prelude enum variant metadata missing".into()))?;
        if selected.fields.len() != payload.len() {
            return Err(Flow::Trap("prelude enum payload shape mismatch".into()));
        }
        let layout = definition.layout.identity;
        let physical_tag = selected.physical_tag;
        self.allocate()?;
        Ok(EvalValue::Enum {
            enum_id,
            variant,
            layout,
            physical_tag,
            payload,
        })
    }

    pub(crate) fn allocate_result(
        &mut self,
        payload: EvalValue,
        is_ok: bool,
    ) -> std::result::Result<EvalValue, Flow> {
        let variant = if is_ok {
            crate::prelude_contract::RESULT_OK_ID
        } else {
            crate::prelude_contract::RESULT_ERR_ID
        };
        self.allocate_enum(crate::prelude_contract::RESULT_ID, variant, vec![payload])
    }

    pub(crate) fn allocate_option(
        &mut self,
        payload: Option<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        let (variant, payload) = match payload {
            Some(value) => (crate::prelude_contract::OPTION_SOME_ID, vec![value]),
            None => (crate::prelude_contract::OPTION_NONE_ID, Vec::new()),
        };
        self.allocate_enum(crate::prelude_contract::OPTION_ID, variant, payload)
    }

    pub(crate) fn allocate_result_error(
        &mut self,
        message: &str,
    ) -> std::result::Result<EvalValue, Flow> {
        let code = self.allocate_option(None)?;
        let detail = self.allocate_string(message.to_owned())?;
        let detail = self.allocate_option(Some(detail))?;
        let error = self.allocate_enum(
            crate::prelude_contract::SYSTEM_ERROR_ID,
            crate::prelude_contract::SYSTEM_UNSUPPORTED_ID,
            vec![code, detail],
        )?;
        self.allocate_result(error, false)
    }
}
