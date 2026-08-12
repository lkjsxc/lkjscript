use super::*;

impl InstallableImage {
    pub fn validate_integrity(&self) -> Result<(), ImageIntegrityError> {
        if self.bytes.is_empty() {
            return Err(ImageIntegrityError::EmptyCode);
        }
        if usize::try_from(self.accounting.code_bytes).ok() != Some(self.bytes.len()) {
            return Err(ImageIntegrityError::CodeAccountingMismatch);
        }
        let expected_metadata = metadata_bytes(MetadataSlices {
            static_bytes: &self.static_bytes,
            entries: &self.entries,
            relocations: &self.relocations,
            runtime_calls: &self.runtime_calls,
            frames: &self.frames,
            heap_runtime_sites: &self.heap_runtime_sites,
            structural_runtime_sites: &self.structural_runtime_sites,
            source_map: &self.source_map,
            trap_map: &self.trap_map,
            outcome_map: &self.outcome_map,
        })
        .ok_or(ImageIntegrityError::MetadataAccountingMismatch)?;
        if expected_metadata != self.accounting.metadata_bytes {
            return Err(ImageIntegrityError::MetadataAccountingMismatch);
        }

        let mut functions = HashSet::new();
        for entry in &self.entries {
            if entry.offset >= entry.end
                || usize::try_from(entry.end).map_or(true, |end| end > self.bytes.len())
            {
                return Err(ImageIntegrityError::EntryRange);
            }
            if !functions.insert(entry.function) {
                return Err(ImageIntegrityError::DuplicateEntry);
            }
        }
        if self.entries.is_empty() {
            return Err(ImageIntegrityError::EntryRange);
        }

        let runtime_calls: HashSet<_> = self.runtime_calls.iter().copied().collect();
        if runtime_calls.len() != self.runtime_calls.len() {
            return Err(ImageIntegrityError::RuntimeCallSet);
        }
        self.validate_execution_domain(&runtime_calls)?;
        let mut relocated_runtime_calls = HashSet::new();
        for relocation in &self.relocations {
            let start = usize::try_from(relocation.offset)
                .map_err(|_| ImageIntegrityError::RelocationRange)?;
            let end = start
                .checked_add(relocation.kind.width())
                .ok_or(ImageIntegrityError::RelocationRange)?;
            if end > self.bytes.len() {
                return Err(ImageIntegrityError::RelocationRange);
            }
            match relocation.target {
                RelocationTarget::Function(function) => {
                    if !functions.contains(&function) {
                        return Err(ImageIntegrityError::RelocationTarget);
                    }
                }
                RelocationTarget::Runtime(slot) => {
                    if !runtime_calls.contains(&slot) {
                        return Err(ImageIntegrityError::RelocationTarget);
                    }
                    relocated_runtime_calls.insert(slot);
                }
            }
        }
        if relocated_runtime_calls != runtime_calls {
            return Err(ImageIntegrityError::RuntimeCallSet);
        }
        let mut frame_functions = HashSet::new();
        for frame in &self.frames {
            let entry = self
                .entries
                .iter()
                .find(|entry| entry.function == frame.function)
                .ok_or(ImageIntegrityError::FrameFacts)?;
            if !frame_functions.insert(frame.function)
                || frame.frame_bytes == 0
                || frame.frame_bytes % 16 != 0
                || frame.uses_red_zone
                || !frame.call_site_aligned_16
                || !valid_frame_homes(frame, entry)
            {
                return Err(ImageIntegrityError::FrameFacts);
            }
        }
        if self.frames.len() != self.entries.len() {
            return Err(ImageIntegrityError::FrameFacts);
        }
        super::heap_sites::verify_heap_sites(self, &runtime_calls)?;
        for (expected_id, site) in self.structural_runtime_sites.iter().enumerate() {
            if usize::try_from(site.id).ok() != Some(expected_id)
                || !functions.contains(&site.function)
                || !site.descriptor.canonical()
            {
                return Err(ImageIntegrityError::StructuralRuntimeSite);
            }
        }
        if self.structural_runtime_sites.is_empty()
            == runtime_calls.contains(&RuntimeCallSlot::StructuralDispatch)
        {
            return Err(ImageIntegrityError::StructuralRuntimeSite);
        }
        for source in &self.source_map {
            if source.code_start >= source.code_end
                || !range_in_function(
                    &self.entries,
                    source.function,
                    source.code_start,
                    source.code_end,
                )
            {
                return Err(ImageIntegrityError::SourceMap);
            }
        }
        let mut explicit_sites = HashSet::new();
        for trap in &self.trap_map {
            if !offset_in_function(&self.entries, trap.function, trap.code_offset)
                || trap.site.is_some() && trap.trap != TrapCode::Explicit
                || trap.site.is_some_and(|site| !explicit_sites.insert(site))
            {
                return Err(ImageIntegrityError::TrapMap);
            }
        }
        for outcome in &self.outcome_map {
            if !offset_in_function(&self.entries, outcome.function, outcome.code_offset) {
                return Err(ImageIntegrityError::OutcomeMap);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BackendLimits, MachinePlanBuilder, RuntimeCallSlot, Signature, SourceFunctionId};

    #[test]
    fn rejects_corrupted_accounting_and_relocation_metadata(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut plan = MachinePlanBuilder::new();
        let function = plan.declare_function(
            SourceFunctionId::new(1),
            Signature::new(Vec::new(), ValueType::I64)?,
        )?;
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let value = builder.i64_const(entry, 21)?;
        let result = builder.runtime_call(entry, RuntimeCallSlot::IdentityI64, vec![value])?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;

        let mut image = crate::encode(plan.verify(BackendLimits::default())?)?;
        assert_eq!(image.validate_integrity(), Ok(()));

        let code_bytes = image.accounting.code_bytes;
        image.accounting.code_bytes += 1;
        assert_eq!(
            image.validate_integrity(),
            Err(ImageIntegrityError::CodeAccountingMismatch)
        );
        image.accounting.code_bytes = code_bytes;
        assert_eq!(image.validate_integrity(), Ok(()));

        assert!(!image.relocations.is_empty());
        image.relocations[0].offset = u32::MAX;
        assert_eq!(
            image.validate_integrity(),
            Err(ImageIntegrityError::RelocationRange)
        );
        Ok(())
    }
}
