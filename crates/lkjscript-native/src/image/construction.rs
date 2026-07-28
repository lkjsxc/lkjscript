use super::*;

impl InstallableImage {
    pub(crate) fn new(parts: ImageParts) -> Result<Self, ImageIntegrityError> {
        let metadata_bytes = metadata_bytes(MetadataSlices {
            static_bytes: &parts.static_bytes,
            entries: &parts.entries,
            relocations: &parts.relocations,
            runtime_calls: &parts.runtime_calls,
            frames: &parts.frames,
            safepoints: &parts.safepoints,
            root_requirements: &parts.root_requirements,
            heap_runtime_sites: &parts.heap_runtime_sites,
            source_map: &parts.source_map,
            trap_map: &parts.trap_map,
            outcome_map: &parts.outcome_map,
        })
        .ok_or(ImageIntegrityError::MetadataAccountingMismatch)?;
        let code_bytes = u64::try_from(parts.bytes.len())
            .map_err(|_| ImageIntegrityError::CodeAccountingMismatch)?;
        let image = Self {
            bytes: parts.bytes.into_boxed_slice(),
            static_bytes: parts.static_bytes.into_boxed_slice(),
            entries: parts.entries.into_boxed_slice(),
            relocations: parts.relocations.into_boxed_slice(),
            runtime_calls: parts.runtime_calls.into_boxed_slice(),
            execution_domain: parts.execution_domain,
            frames: parts.frames.into_boxed_slice(),
            safepoints: parts.safepoints.into_boxed_slice(),
            root_requirements: parts.root_requirements.into_boxed_slice(),
            heap_runtime_sites: parts.heap_runtime_sites.into_boxed_slice(),
            source_map: parts.source_map.into_boxed_slice(),
            trap_map: parts.trap_map.into_boxed_slice(),
            outcome_map: parts.outcome_map.into_boxed_slice(),
            accounting: CodeAccounting {
                code_bytes,
                metadata_bytes,
                work_units: parts.work_units,
            },
            contracts: parts.contracts,
        };
        image.validate_integrity()?;
        Ok(image)
    }
}
