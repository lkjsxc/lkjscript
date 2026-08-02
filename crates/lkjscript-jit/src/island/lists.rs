use crate::*;
use lkjscript_executable::NativeRuntimeServices;

impl NativeRuntimeServices for JitIslandServices {
    fn heap_operation(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        let descriptor = site.descriptor();
        match descriptor.operation() {
            HeapOperation::EmptyList => {
                let value_type = descriptor
                    .result_type()
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                Ok(NativeValue::Reference(
                    lkjscript_native::NativeReference::new(value_type, 0),
                ))
            }
            HeapOperation::Cons => self.list_prepend(descriptor, arguments),
            HeapOperation::Car => self.list_first(descriptor, arguments),
            HeapOperation::Cdr => self.list_rest(descriptor, arguments),
            HeapOperation::IsEmptyList => self.list_is_empty(arguments),
            HeapOperation::ListEqual => self.list_equal(arguments),
            _ => self.list_trap("unsupported heap operation entered structural island"),
        }
    }
}

impl JitIslandServices {
    fn list_prepend(
        &mut self,
        descriptor: &lkjscript_native::HeapCallDescriptor,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        let [head, NativeValue::Reference(tail)] = arguments else {
            return self.list_trap("structural list prepend arguments mismatch");
        };
        let result_type = descriptor
            .result_type()
            .reference_type()
            .ok_or(NativeServiceError::HostFailure)?;
        if tail.reference_type() != result_type
            || self.list_allocations >= self.max_list_allocations
        {
            return self.list_trap("structural list prepend type or allocation mismatch");
        }
        let tail_key = self.list_key(*tail)?;
        let projected = self
            .lists
            .reserved_bytes_estimate()
            .saturating_add(self.lists.prepend_storage_increase());
        if projected > self.max_runtime_bytes {
            return Err(NativeServiceError::ResourceLimitExceeded);
        }
        let head_type = descriptor
            .input_types()
            .first()
            .copied()
            .ok_or(NativeServiceError::HostFailure)?;
        if matches!(head, NativeValue::StructuralOwner(_)) {
            self.list_owners
                .try_reserve(1)
                .map_err(|_| NativeServiceError::ResourceLimitExceeded)?;
        }
        let (retained, owner) = self.retain_list_value(*head, head_type)?;
        let key =
            match self
                .lists
                .prepend_typed(retained, tail_key, reference_layout_key(result_type))
            {
                Ok(key) => key,
                Err(error) => {
                    if owner {
                        let _ = self.structural.release_list_owner(retained);
                    }
                    return self.list_error(error);
                }
            };
        if owner {
            self.list_owners.push(retained);
        }
        self.list_allocations = self.list_allocations.saturating_add(1);
        Ok(NativeValue::Reference(
            lkjscript_native::NativeReference::new(result_type, key.to_word()),
        ))
    }

    fn list_first(
        &mut self,
        descriptor: &lkjscript_native::HeapCallDescriptor,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        let key = self.only_list_key(arguments)?;
        let value = self.lists.first_cloned(key).map_err(Self::map_list_error)?;
        self.native_from_list_value(value, descriptor.result_type())
    }

    fn list_rest(
        &mut self,
        descriptor: &lkjscript_native::HeapCallDescriptor,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        let key = self.only_list_key(arguments)?;
        let tail = self.lists.rest(key).map_err(Self::map_list_error)?;
        let value_type = descriptor
            .result_type()
            .reference_type()
            .ok_or(NativeServiceError::HostFailure)?;
        Ok(NativeValue::Reference(
            lkjscript_native::NativeReference::new(value_type, tail.to_word()),
        ))
    }

    fn list_is_empty(
        &mut self,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        Ok(NativeValue::Bool(self.only_list_key(arguments)?.is_empty()))
    }

    fn list_equal(&mut self, arguments: &[NativeValue]) -> Result<NativeValue, NativeServiceError> {
        let [NativeValue::Reference(left), NativeValue::Reference(right)] = arguments else {
            return self.list_trap("structural list equality arguments mismatch");
        };
        if left.reference_type() != right.reference_type() {
            return self.list_trap("structural list equality layout mismatch");
        }
        let left = self.list_key(*left)?;
        let right = self.list_key(*right)?;
        self.compare_lists(left, right).map(NativeValue::Bool)
    }

    fn only_list_key(
        &mut self,
        arguments: &[NativeValue],
    ) -> Result<lkjscript_core::SegmentedListKey, NativeServiceError> {
        let [NativeValue::Reference(value)] = arguments else {
            return self.list_trap("structural list argument mismatch");
        };
        self.list_key(*value)
    }

    pub(super) fn list_key(
        &mut self,
        value: lkjscript_native::NativeReference,
    ) -> Result<lkjscript_core::SegmentedListKey, NativeServiceError> {
        let key = self
            .lists
            .key_from_word(value.opaque_word())
            .map_err(Self::map_list_error)?;
        self.lists
            .validate_type(key, reference_layout_key(value.reference_type()))
            .map_err(Self::map_list_error)?;
        Ok(key)
    }

    fn list_error<T>(
        &mut self,
        error: lkjscript_core::SegmentedListError,
    ) -> Result<T, NativeServiceError> {
        self.structural
            .record_trap(format!("structural list operation failed: {error:?}"));
        Err(Self::map_list_error(error))
    }

    pub(super) fn map_list_error(error: lkjscript_core::SegmentedListError) -> NativeServiceError {
        match error {
            lkjscript_core::SegmentedListError::Limit(_) => {
                NativeServiceError::ResourceLimitExceeded
            }
            _ => NativeServiceError::Trap,
        }
    }

    fn list_trap<T>(&mut self, message: &str) -> Result<T, NativeServiceError> {
        self.structural.record_trap(message);
        Err(NativeServiceError::Trap)
    }

    pub(super) fn release_list_owners(&mut self) {
        for value in std::mem::take(&mut self.list_owners) {
            if self.structural.release_list_owner(value).is_err() {
                self.structural
                    .record_trap("structural list owner teardown failed");
                break;
            }
        }
    }
}
