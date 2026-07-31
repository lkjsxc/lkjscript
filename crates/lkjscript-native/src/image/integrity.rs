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

        if self.static_bytes.len() > u32::MAX as usize {
            return Err(ImageIntegrityError::StaticBytes);
        }
        let mut functions = HashSet::new();
        for entry in &self.entries {
            if entry.offset >= entry.end || entry.end as usize > self.bytes.len() {
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
            let start = relocation.offset as usize;
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
            if site.id as usize != expected_id
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
