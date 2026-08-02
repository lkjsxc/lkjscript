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
        let mut has_invocation_reference = false;
        let mut has_resource = false;
        let mut has_unique = false;
        let mut has_structural = false;
        for value_type in value_types {
            has_reference |= matches!(value_type, ValueType::Reference(_));
            has_invocation_reference |= matches!(
                value_type,
                ValueType::Reference(
                    ReferenceType::List(_, _, _, _) | ReferenceType::RegionProduct(_, _)
                )
            );
            has_resource |= matches!(
                value_type,
                ValueType::Capability(_) | ValueType::Resource(_)
            );
            has_unique |= matches!(
                value_type,
                ValueType::StaticBytes | ValueType::Unique(_) | ValueType::Loan(_)
            );
            has_structural |= matches!(
                value_type,
                ValueType::StructuralOwner(_)
                    | ValueType::StructuralView(_)
                    | ValueType::StructuralDestination(_)
            );
        }
        let has_island_value = has_resource || has_unique || has_structural;
        match self.execution_domain {
            NativeExecutionDomain::CollectorFree
                if (has_reference && !has_invocation_reference)
                    || (has_resource && (has_unique || has_structural)) =>
            {
                Err(ImageIntegrityError::ExecutionDomain)
            }
            NativeExecutionDomain::InvocationRegion
                if !has_invocation_reference
                    || has_island_value
                    || runtime_calls.contains(&RuntimeCallSlot::StdinHandle) =>
            {
                Err(ImageIntegrityError::ExecutionDomain)
            }
            _ => Ok(()),
        }
    }
}
