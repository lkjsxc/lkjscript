#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutableLimits {
    pub(super) max_object_code_bytes: u64,
    pub(super) max_object_metadata_bytes: u64,
    pub(super) max_object_work_units: u64,
    pub(super) max_total_code_bytes: u64,
    pub(super) max_total_metadata_bytes: u64,
    pub(super) max_total_work_units: u64,
    pub(super) max_objects: u64,
}

impl ExecutableLimits {
    #[must_use]
    pub const fn new(
        max_object_code_bytes: u64,
        max_object_metadata_bytes: u64,
        max_object_work_units: u64,
        max_total_code_bytes: u64,
        max_total_metadata_bytes: u64,
        max_total_work_units: u64,
        max_objects: u64,
    ) -> Self {
        Self {
            max_object_code_bytes,
            max_object_metadata_bytes,
            max_object_work_units,
            max_total_code_bytes,
            max_total_metadata_bytes,
            max_total_work_units,
            max_objects,
        }
    }
}

impl Default for ExecutableLimits {
    fn default() -> Self {
        Self::new(
            4 * 1024 * 1024,
            4 * 1024 * 1024,
            100_000,
            64 * 1024 * 1024,
            64 * 1024 * 1024,
            1_000_000,
            1_024,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExecutableUsage {
    pub(super) code_bytes: u64,
    pub(super) metadata_bytes: u64,
    pub(super) work_units: u64,
    pub(super) objects: u64,
}

impl ExecutableUsage {
    #[must_use]
    pub const fn code_bytes(self) -> u64 {
        self.code_bytes
    }

    #[must_use]
    pub const fn metadata_bytes(self) -> u64 {
        self.metadata_bytes
    }

    #[must_use]
    pub const fn work_units(self) -> u64 {
        self.work_units
    }

    #[must_use]
    pub const fn objects(self) -> u64 {
        self.objects
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutableLimitKind {
    ObjectCodeBytes,
    ObjectMetadataBytes,
    ObjectWorkUnits,
    TotalCodeBytes,
    TotalMetadataBytes,
    TotalWorkUnits,
    ObjectCount,
}
