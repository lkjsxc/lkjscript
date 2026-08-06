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

pub(super) const MAX_NATIVE_ENTRY_COUNTS: usize = 64;
pub(super) const MAX_ACTIVE_FRAMES: usize = 64;
pub(super) const DEFAULT_MAX_NATIVE_STACK_BYTES: usize = 4 * 1024 * 1024;
pub(super) const DEFAULT_MAX_NATIVE_FRAME_BYTES: usize = 1024 * 1024;
pub(super) const NATIVE_STACK_GUARD_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy)]
pub(super) struct ActiveFrame {
    pub(super) function_ordinal: Option<u64>,
    pub(super) rbp: *mut u8,
    pub(super) reserved_bytes: usize,
    pub(super) value_homes: usize,
}

const EMPTY_ACTIVE_FRAME: ActiveFrame = ActiveFrame {
    function_ordinal: None,
    rbp: std::ptr::null_mut(),
    reserved_bytes: 0,
    value_homes: 0,
};

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
    pub(super) heap_arguments: Vec<NativeValue>,
    pub(super) heap_operation_attempts: u64,
    pub(super) heap_operation_successes: u64,
    pub(super) metadata_invalid: bool,
}
