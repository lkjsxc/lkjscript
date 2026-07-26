use super::*;

mod collection;
mod frames;
mod heap;
mod lifecycle;
mod roots;
mod stack;

pub(super) const MAX_NATIVE_ENTRY_COUNTS: usize = 64;
pub(super) const MAX_ACTIVE_FRAMES: usize = 64;
pub(super) const MAX_MATERIALIZED_ROOTS: usize = 65_536;
pub(super) const MAX_COLLECTION_REPORTS: usize = 65_536;
pub(super) const DEFAULT_MAX_NATIVE_STACK_BYTES: usize = 4 * 1024 * 1024;
pub(super) const DEFAULT_MAX_NATIVE_FRAME_BYTES: usize = 1024 * 1024;
pub(super) const NATIVE_STACK_GUARD_BYTES: usize = 16 * 1024;
pub(super) const INVALID_SAFEPOINT: u32 = u32::MAX;

#[derive(Clone, Copy)]
pub(super) struct ActiveFrame {
    pub(super) function_ordinal: u32,
    pub(super) rbp: *mut u8,
    pub(super) safepoint: u32,
    pub(super) reserved_bytes: usize,
    pub(super) value_homes: usize,
}

const EMPTY_ACTIVE_FRAME: ActiveFrame = ActiveFrame {
    function_ordinal: u32::MAX,
    rbp: std::ptr::null_mut(),
    safepoint: INVALID_SAFEPOINT,
    reserved_bytes: 0,
    value_homes: 0,
};

#[derive(Clone, Copy)]
pub(super) struct PendingFrameReservation {
    pub(super) function_ordinal: u32,
    pub(super) rbp: *mut u8,
    pub(super) frame_bytes: usize,
    pub(super) value_homes: usize,
}

#[derive(Clone, Copy)]
pub(super) struct RootAddress {
    pub(super) address: *mut u64,
    pub(super) original_word: u64,
    pub(super) reference_type: ReferenceType,
    pub(super) frame_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MaterializeRootError {
    InvalidFrame,
    Capacity,
}

#[repr(C)]
pub(super) struct NativeCallState<'a> {
    // These first three fields are the stable runtime ABI consumed directly by
    // generated code. The native contract adds frame operations without changing the
    // semantic or runtime ABI versions.
    pub(super) status: u32,
    pub(super) trap: u32,
    pub(super) payload: i64,
    pub(super) _scratch_integer_arguments: [u64; 2],
    pub(super) _scratch_float_arguments: [u64; 2],
    pub(super) poll_fuel_remaining: u64,
    pub(super) deadline_ms: i64,
    pub(super) poll_count: u64,
    pub(super) native_entries: [u64; MAX_NATIVE_ENTRY_COUNTS],
    pub(super) image: &'a InstallableImage,
    pub(super) services: &'a mut dyn NativeRuntimeServices,
    pub(super) active_frames: [ActiveFrame; MAX_ACTIVE_FRAMES],
    pub(super) active_depth: usize,
    pub(super) maximum_active_frames: usize,
    pub(super) maximum_active_values: usize,
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
    pub(super) collection_calls: u64,
    pub(super) maximum_roots: usize,
    pub(super) exact_root_counts: Vec<usize>,
    pub(super) roots: Vec<NativeRoot>,
    pub(super) root_addresses: Vec<RootAddress>,
    pub(super) heap_arguments: Vec<NativeValue>,
    pub(super) heap_operation_attempts: u64,
    pub(super) heap_operation_successes: u64,
    pub(super) barrier_count: u64,
    pub(super) metadata_invalid: bool,
}
