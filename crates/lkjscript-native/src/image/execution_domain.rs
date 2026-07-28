use super::*;

impl InstallableImage {
    pub(super) fn validate_execution_domain(
        &self,
        runtime_calls: &HashSet<RuntimeCallSlot>,
    ) -> Result<(), ImageIntegrityError> {
        let value_types = self
            .entries
            .iter()
            .flat_map(|entry| {
                entry
                    .signature
                    .parameters()
                    .iter()
                    .copied()
                    .chain(std::iter::once(entry.signature.result()))
            })
            .chain(
                self.frames
                    .iter()
                    .flat_map(|frame| frame.homes.iter().map(|home| home.value_type)),
            );
        let mut has_reference = false;
        let mut has_resource = false;
        let mut has_unique = false;
        for value_type in value_types {
            has_reference |= matches!(value_type, ValueType::Reference(_));
            has_resource |= matches!(
                value_type,
                ValueType::Capability(_) | ValueType::Resource(_)
            );
            has_unique |= matches!(
                value_type,
                ValueType::StaticBytes | ValueType::Unique(_) | ValueType::Loan(_)
            );
        }
        let has_island_value = has_resource || has_unique;
        match self.execution_domain {
            NativeExecutionDomain::CollectorFree
                if has_reference
                    || (has_resource && has_unique)
                    || !self.safepoints.is_empty()
                    || !self.root_requirements.is_empty()
                    || !self.heap_runtime_sites.is_empty()
                    || runtime_calls.contains(&RuntimeCallSlot::CollectReference)
                    || runtime_calls.contains(&RuntimeCallSlot::HeapDispatch)
                    || runtime_calls.contains(&RuntimeCallSlot::PublishSafepoint) =>
            {
                Err(ImageIntegrityError::ExecutionDomain)
            }
            NativeExecutionDomain::LegacyHeap
                if has_island_value || runtime_calls.contains(&RuntimeCallSlot::StdinHandle) =>
            {
                Err(ImageIntegrityError::ExecutionDomain)
            }
            _ => Ok(()),
        }
    }
}
