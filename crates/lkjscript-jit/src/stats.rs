use crate::*;

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
pub struct FunctionTierRecord {
    pub(crate) function: FunctionId,
    pub(crate) name: String,
    pub(crate) state: TierState,
    pub(crate) call_count: u64,
    pub(crate) attempts: u8,
    pub(crate) last_failure: Option<FailureCode>,
    pub(crate) code_object: Option<u64>,
    pub(crate) epoch: u64,
    pub(crate) native_entries: u64,
    pub(crate) auto_entry_eligible: bool,
}

impl FunctionTierRecord {
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn state(&self) -> TierState {
        self.state
    }

    pub const fn call_count(&self) -> u64 {
        self.call_count
    }

    pub const fn attempts(&self) -> u8 {
        self.attempts
    }

    pub const fn last_failure(&self) -> Option<FailureCode> {
        self.last_failure
    }

    pub const fn code_object(&self) -> Option<u64> {
        self.code_object
    }

    pub const fn epoch(&self) -> u64 {
        self.epoch
    }

    pub const fn native_entries(&self) -> u64 {
        self.native_entries
    }

    pub const fn auto_entry_eligible(&self) -> bool {
        self.auto_entry_eligible
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileStats {
    pub(crate) optimization: Duration,
    pub(crate) lowering_and_encoding: Duration,
    pub(crate) installation: Duration,
    pub(crate) work_units: u64,
}

impl CompileStats {
    pub const fn optimization(&self) -> Duration {
        self.optimization
    }

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
    pub functions: Vec<FunctionTierRecord>,
    pub code_objects: Vec<CodeObjectRecord>,
    pub native_entries: u64,
    pub direct_native_calls: u64,
    pub poll_calls: u64,
    pub vm_fallbacks: u64,
    pub compile_failures: u64,
    pub native_invocations: u64,
    pub time_to_first_native_entry: Option<Duration>,
    pub first_native_call: Option<Duration>,
    pub native_execution: Duration,
    pub auto_threshold: u64,
    pub auto_enabled: bool,
    pub code_cache_peak_objects: u64,
    pub code_cache_peak_bytes: u64,
    pub metadata_cache_peak_bytes: u64,
    pub accounted_allocation_peak_bytes: u64,
    pub allocations: u64,
    pub allocation_bytes_estimate: u64,
    pub collections: u64,
    pub peak_live_heap_bytes_estimate: usize,
    pub maximum_roots: usize,
    pub runtime_heap_attempts: u64,
    pub runtime_heap_successes: u64,
    pub barrier_count: u64,
    pub collector_runtime_invocations: u64,
    pub resource_runtime_calls: u64,
    pub native_resources: NativeResourceStats,
    pub peak_native_frame_depth: usize,
    pub vm_to_native_transitions: u64,
    pub native_to_vm_transitions: u64,
    pub baseline_native_entries: u64,
    pub optimizing_native_entries: u64,
    pub baseline_code_objects: u64,
    pub optimizing_code_objects: u64,
    pub optimizing_passes: u64,
    pub optimization_discovery_passes: u64,
    pub optimization_checker_passes: u64,
    pub optimization_reconstruction_passes: u64,
    pub optimization_cleanup_passes: u64,
    pub optimization_validation_passes: u64,
    pub optimization_certificate_records: u64,
    pub optimization_certificate_bytes_estimate: u64,
    pub algebraic_rewrites: u64,
    pub gvn_rewrites: u64,
    pub checked_i64_rewrites: u64,
}
