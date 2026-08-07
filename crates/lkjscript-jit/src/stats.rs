use crate::*;

mod structural;
mod unique;
pub use structural::NativeStructuralStats;
pub use unique::NativeUniqueStats;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeResourceStats {
    pub reservations: u64,
    pub borrowed_installs: u64,
    pub borrowed_reuses: u64,
    pub borrowed_removals: u64,
    pub explicit_closes: u64,
    pub slot_reuses: u64,
    pub cleanup_attempts: u64,
    pub ordinary_obligations: u64,
    pub borrowed_obligations: u64,
    pub emergency_obligations: u64,
    pub teardown_failures: u64,
}

impl NativeResourceStats {
    pub(crate) fn add(&mut self, other: Self) {
        self.reservations = self.reservations.saturating_add(other.reservations);
        self.borrowed_installs = self
            .borrowed_installs
            .saturating_add(other.borrowed_installs);
        self.borrowed_reuses = self.borrowed_reuses.saturating_add(other.borrowed_reuses);
        self.borrowed_removals = self
            .borrowed_removals
            .saturating_add(other.borrowed_removals);
        self.explicit_closes = self.explicit_closes.saturating_add(other.explicit_closes);
        self.slot_reuses = self.slot_reuses.saturating_add(other.slot_reuses);
        self.cleanup_attempts = self.cleanup_attempts.saturating_add(other.cleanup_attempts);
        self.ordinary_obligations = other.ordinary_obligations;
        self.borrowed_obligations = other.borrowed_obligations;
        self.emergency_obligations = other.emergency_obligations;
        self.teardown_failures = self
            .teardown_failures
            .saturating_add(other.teardown_failures);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileStats {
    pub(crate) lowering_and_encoding: Duration,
    pub(crate) installation: Duration,
    pub(crate) work_units: u64,
}

impl CompileStats {
    pub const fn lowering_and_encoding(&self) -> Duration {
        self.lowering_and_encoding
    }

    pub const fn installation(&self) -> Duration {
        self.installation
    }

    pub const fn work_units(&self) -> u64 {
        self.work_units
    }
}

#[derive(Debug)]
pub struct JitStats {
    pub code_objects: Vec<CodeObjectRecord>,
    pub native_entries: u64,
    pub direct_native_calls: u64,
    pub poll_calls: u64,
    pub native_invocations: u64,
    pub time_to_first_native_entry: Option<Duration>,
    pub first_native_call: Option<Duration>,
    pub native_execution: Duration,
    pub runtime_heap_attempts: u64,
    pub runtime_heap_successes: u64,
    pub segmented_lists: lkjscript_core::SegmentedListMetrics,
    pub segmented_list_reserved_bytes_estimate: Option<u64>,
    pub region_products: Option<lkjscript_core::RegionProductMetrics>,
    pub resource_runtime_calls: u64,
    pub native_resources: NativeResourceStats,
    pub native_unique: NativeUniqueStats,
    pub native_structural: NativeStructuralStats,
    pub unique_runtime_calls: u64,
    pub structural_runtime_calls: u64,
    pub peak_native_frame_depth: usize,
    pub peak_native_stack_bytes: usize,
}
