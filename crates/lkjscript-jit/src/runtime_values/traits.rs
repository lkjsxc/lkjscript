use crate::*;

impl NativeRuntimeServices for JitValueServices<'_> {
    fn heap_operation(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        if arguments.len() != site.descriptor().input_types().len()
            || arguments
                .iter()
                .zip(site.descriptor().input_types())
                .any(|(value, expected)| value.value_type() != *expected)
        {
            return self.trap("heap runtime argument metadata mismatch");
        }
        for value in arguments {
            if let NativeValue::Reference(reference) = value {
                self.reference_value(*reference).map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })?;
            }
        }
        self.execute(site, arguments)
    }
}
