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

    pub(crate) fn allocate_result(
        &mut self,
        payload: EvalValue,
        is_ok: bool,
    ) -> std::result::Result<EvalValue, Flow> {
        self.allocate()?;
        if is_ok {
            Ok(EvalValue::Ok(Box::new(payload)))
        } else {
            Ok(EvalValue::Err(Box::new(payload)))
        }
    }

    pub(crate) fn allocate_result_error(
        &mut self,
        message: &str,
    ) -> std::result::Result<EvalValue, Flow> {
        let payload = self.allocate_string(message.to_owned())?;
        self.allocate_result(payload, false)
    }
}
