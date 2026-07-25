use super::*;

impl InstallableImage {
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn entries(&self) -> &[EntryMetadata] {
        &self.entries
    }

    #[must_use]
    pub fn relocations(&self) -> &[Relocation] {
        &self.relocations
    }

    #[must_use]
    pub fn runtime_calls(&self) -> &[RuntimeCallSlot] {
        &self.runtime_calls
    }

    #[must_use]
    pub fn frames(&self) -> &[FrameFacts] {
        &self.frames
    }

    #[must_use]
    pub fn safepoints(&self) -> &[Safepoint] {
        &self.safepoints
    }

    #[must_use]
    pub fn heap_runtime_sites(&self) -> &[HeapRuntimeSite] {
        &self.heap_runtime_sites
    }

    #[must_use]
    pub fn source_map(&self) -> &[SourceMapEntry] {
        &self.source_map
    }

    #[must_use]
    pub fn trap_map(&self) -> &[TrapMapEntry] {
        &self.trap_map
    }

    #[must_use]
    pub fn outcome_map(&self) -> &[OutcomeMapEntry] {
        &self.outcome_map
    }

    #[must_use]
    pub const fn accounting(&self) -> CodeAccounting {
        self.accounting
    }

    #[must_use]
    pub const fn versions(&self) -> AbiVersions {
        self.versions
    }
}
