use super::*;

mod frames;
mod island;
mod island_frames;
mod island_runtime_values;
mod island_stack;
mod lifecycle;
mod runtime_values;
mod stack;

pub(super) use island::*;

pub(super) const DEFAULT_MAX_NATIVE_STACK_BYTES: usize = 4 * 1024 * 1024;
pub(super) const DEFAULT_MAX_NATIVE_FRAME_BYTES: usize = 1024 * 1024;
pub(super) const NATIVE_STACK_GUARD_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy)]
pub(super) struct ActiveFrame {
    pub(super) function_ordinal: u64,
    pub(super) rbp: *mut u8,
    pub(super) reserved_bytes: usize,
    pub(super) value_homes: usize,
}

#[derive(Clone, Copy)]
pub(super) struct PendingFrameReservation {
    pub(super) function_ordinal: u64,
    pub(super) rbp: *mut u8,
    pub(super) frame_bytes: usize,
    pub(super) value_homes: usize,
}

#[repr(C)]
pub(super) struct NativeCallState<'a> {
    // These first four fields are the stable runtime ABI consumed directly by
    // generated code. `trap_site_present` makes the full-width site optional without
    // reserving a numeric sentinel.
    pub(super) status: u32,
    pub(super) trap: u32,
    pub(super) payload: i64,
    pub(super) trap_site_present: u64,
    pub(super) _scratch_integer_arguments: [u64; 5],
    pub(super) _scratch_float_arguments: [u64; 2],
    pub(super) poll_fuel_remaining: Option<u64>,
    pub(super) deadline_ms: i64,
    pub(super) poll_count: u64,
    pub(super) native_entries: Vec<NativeEntryCount>,
    pub(super) entry_mapping: &'a NativeEntryMapping,
    pub(super) image: &'a InstallableImage,
    pub(super) services: &'a mut dyn NativeRuntimeServices,
    pub(super) active_frames: Vec<ActiveFrame>,
    pub(super) maximum_active_frames: Option<usize>,
    pub(super) maximum_active_values: Option<usize>,
    pub(super) maximum_native_stack_bytes: usize,
    pub(super) maximum_native_frame_bytes: usize,
    pub(super) native_stack_low: usize,
    pub(super) native_stack_high: usize,
    pub(super) pending_reservation: Option<PendingFrameReservation>,
    pub(super) reserved_native_stack_bytes: usize,
    pub(super) peak_native_stack_bytes: usize,
    pub(super) peak_active_depth: usize,
    pub(super) active_value_homes: usize,
    pub(super) peak_active_value_homes: usize,
    pub(super) heap_arguments: Vec<NativeValue>,
    pub(super) heap_operation_attempts: u64,
    pub(super) heap_operation_successes: u64,
    pub(super) invalid_entry_accounting: Option<u64>,
    pub(super) bookkeeping_allocation_failed: bool,
    pub(super) metadata_invalid: bool,
}

#[derive(Debug)]
pub(super) struct NativeEntryMapping {
    by_source: Vec<(u64, usize)>,
}

impl NativeEntryMapping {
    pub(super) fn try_new(image: &InstallableImage) -> Result<Self, InstallError> {
        let mut by_source = Vec::new();
        by_source
            .try_reserve_exact(image.entries().len())
            .map_err(|_| InstallError::AllocationFailed)?;
        for (ordinal, entry) in image.entries().iter().enumerate() {
            by_source.push((entry.source_function().get(), ordinal));
        }
        by_source.sort_unstable_by_key(|(source, _)| *source);
        Ok(Self { by_source })
    }

    pub(super) fn ordinal_for_source(&self, source: u64) -> Option<usize> {
        self.by_source
            .binary_search_by_key(&source, |(candidate, _)| *candidate)
            .ok()
            .map(|index| self.by_source[index].1)
    }
}

pub(super) fn try_entry_counts(
    image: &InstallableImage,
) -> Result<Vec<NativeEntryCount>, InvocationError> {
    let mut counts = Vec::new();
    counts
        .try_reserve_exact(image.entries().len())
        .map_err(|_| InvocationError::NativeBookkeepingAllocationFailed)?;
    counts.extend(image.entries().iter().map(|entry| NativeEntryCount {
        source_function: entry.source_function().get(),
        entries: 0,
    }));
    Ok(counts)
}

pub(super) fn record_native_entry(counts: &mut [NativeEntryCount], ordinal: usize) -> bool {
    let Some(count) = counts.get_mut(ordinal) else {
        return false;
    };
    let Some(entries) = count.entries.checked_add(1) else {
        return false;
    };
    count.entries = entries;
    true
}
