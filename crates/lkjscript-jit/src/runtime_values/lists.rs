use super::*;
use crate::*;

impl JitValueServices<'_> {
    pub(crate) fn execute_lists(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        let descriptor = site.descriptor();
        let argument = |index: usize| {
            arguments
                .get(index)
                .copied()
                .ok_or(NativeServiceError::HostFailure)
        };
        let as_reference = |value: NativeValue| match value {
            NativeValue::Reference(reference) => Ok(reference),
            _ => Err(NativeServiceError::HostFailure),
        };
        match descriptor.operation() {
            HeapOperation::Cons => {
                let head = self.value_from_native(argument(0)?)?;
                let tail = as_reference(argument(1)?)?;
                let result_type = descriptor
                    .result_type()
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                if tail.reference_type() != result_type {
                    return self.trap("list-prepend tail layout mismatch");
                }
                let tail_key = self.list_key(tail)?;
                if self.max_list_allocations.is_some_and(|maximum| {
                    self.list_allocations
                        .saturating_add(self.region_product_allocations)
                        >= maximum
                }) {
                    self.last_resource = Some(ResourceLimitKind::Allocations);
                    return Err(NativeServiceError::ResourceLimitExceeded);
                }
                let increase = self.lists.prepend_storage_increase();
                let projected = self
                    .lists
                    .reserved_bytes_estimate()
                    .saturating_add(self.region_products.metrics().reserved_bytes_estimate)
                    .saturating_add(increase);
                if self
                    .max_runtime_bytes
                    .is_some_and(|maximum| projected > maximum)
                {
                    self.last_resource = Some(ResourceLimitKind::HeapBytes);
                    return Err(NativeServiceError::ResourceLimitExceeded);
                }
                let key = self
                    .lists
                    .prepend_typed(head, tail_key, reference_layout_key(result_type))
                    .map_err(|error| self.list_error(error))?;
                self.list_allocations = self.list_allocations.saturating_add(1);
                Ok(NativeValue::Reference(
                    lkjscript_native::NativeReference::new(result_type, key.to_word()),
                ))
            }
            HeapOperation::Car => {
                let list = as_reference(argument(0)?)?;
                let key = self.list_key(list)?;
                let value = self
                    .lists
                    .first_cloned(key)
                    .map_err(|error| self.list_error(error))?;
                self.native_from_value(value, descriptor.result_type())
            }
            HeapOperation::Cdr => {
                let list = as_reference(argument(0)?)?;
                let key = self.list_key(list)?;
                let tail = self
                    .lists
                    .rest(key)
                    .map_err(|error| self.list_error(error))?;
                let result_type = descriptor
                    .result_type()
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                if list.reference_type() != result_type {
                    return self.trap("list-rest result layout mismatch");
                }
                Ok(NativeValue::Reference(
                    lkjscript_native::NativeReference::new(result_type, tail.to_word()),
                ))
            }
            HeapOperation::IsEmptyList => {
                let list = as_reference(argument(0)?)?;
                let key = self.list_key(list)?;
                Ok(NativeValue::Bool(key.is_empty()))
            }
            _ => Err(NativeServiceError::HostFailure),
        }
    }

    pub(crate) fn list_key(
        &mut self,
        reference: lkjscript_native::NativeReference,
    ) -> Result<lkjscript_core::SegmentedListKey, NativeServiceError> {
        let key = self
            .lists
            .key_from_word(reference.opaque_word())
            .map_err(|error| self.list_error(error))?;
        self.lists
            .validate_type(key, reference_layout_key(reference.reference_type()))
            .map_err(|error| self.list_error(error))?;
        Ok(key)
    }

    pub(crate) fn list_error(
        &mut self,
        error: lkjscript_core::SegmentedListError,
    ) -> NativeServiceError {
        match error {
            lkjscript_core::SegmentedListError::Limit(
                lkjscript_core::SegmentedListLimit::HostAllocation
                | lkjscript_core::SegmentedListLimit::Entries
                | lkjscript_core::SegmentedListLimit::Segments,
            ) => {
                self.last_resource = Some(ResourceLimitKind::Allocations);
                NativeServiceError::ResourceLimitExceeded
            }
            _ => {
                self.last_trap = Some(format!("segmented-list operation failed: {error:?}"));
                NativeServiceError::Trap
            }
        }
    }
}
