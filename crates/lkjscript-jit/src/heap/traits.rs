use super::*;
use crate::*;

impl NativeRuntimeServices for JitHeapServices<'_> {
    fn collect_references(&mut self, roots: &mut [NativeRoot]) -> Result<(), NativeServiceError> {
        let values = self.roots(roots)?;
        self.heap.collect(&values);
        Ok(())
    }

    fn prepare_heap_operation(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
        roots: &mut [NativeRoot],
    ) -> Result<bool, NativeServiceError> {
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
                native_reference_value(self.heap, *reference).map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })?;
            }
        }
        let root_values = self.roots(roots)?;
        // The baseline slice deliberately uses the bounded slow path for every
        // source allocation. This collects before publication; stress mode
        // additionally collects at non-allocating heap sites.
        let collected = self.force_collection
            || site.descriptor().allocation() == lkjscript_native::AllocationClass::Bounded;
        if collected {
            self.heap.collect(&root_values);
        }
        Ok(collected)
    }

    fn heap_operation(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        self.execute(site, arguments)
    }
}
