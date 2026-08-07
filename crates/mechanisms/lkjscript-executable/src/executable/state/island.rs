use super::*;

#[derive(Clone, Copy)]
pub(in crate::executable) struct IslandFrame {
    pub(in crate::executable) function_ordinal: u64,
    pub(in crate::executable) rbp: *mut u8,
    pub(in crate::executable) reserved_bytes: usize,
    pub(in crate::executable) value_homes: usize,
}

#[derive(Clone, Copy)]
pub(in crate::executable) struct IslandFrameReservation {
    pub(in crate::executable) function_ordinal: u64,
    pub(in crate::executable) rbp: *mut u8,
    pub(in crate::executable) frame_bytes: usize,
    pub(in crate::executable) value_homes: usize,
}

#[repr(C)]
pub(in crate::executable) struct IslandCallState<'a> {
    pub(in crate::executable) status: u32,
    pub(in crate::executable) trap: u32,
    pub(in crate::executable) payload: i64,
    pub(in crate::executable) trap_site_present: u64,
    pub(in crate::executable) _scratch_integer_arguments: [u64; 5],
    pub(in crate::executable) _scratch_float_arguments: [u64; 2],
    pub(in crate::executable) poll_fuel_remaining: u64,
    pub(in crate::executable) deadline_ms: i64,
    pub(in crate::executable) poll_count: u64,
    pub(in crate::executable) native_entries: Vec<NativeEntryCount>,
    pub(in crate::executable) entry_mapping: &'a NativeEntryMapping,
    pub(in crate::executable) image: &'a InstallableImage,
    pub(in crate::executable) services: &'a mut dyn NativeIslandRuntimeServices,
    pub(in crate::executable) active_frames: Vec<IslandFrame>,
    pub(in crate::executable) maximum_active_frames: usize,
    pub(in crate::executable) maximum_active_values: usize,
    pub(in crate::executable) maximum_native_stack_bytes: usize,
    pub(in crate::executable) maximum_native_frame_bytes: usize,
    pub(in crate::executable) native_stack_low: usize,
    pub(in crate::executable) native_stack_high: usize,
    pub(in crate::executable) pending_reservation: Option<IslandFrameReservation>,
    pub(in crate::executable) reserved_native_stack_bytes: usize,
    pub(in crate::executable) peak_native_stack_bytes: usize,
    pub(in crate::executable) peak_active_depth: usize,
    pub(in crate::executable) active_value_homes: usize,
    pub(in crate::executable) peak_active_value_homes: usize,
    pub(in crate::executable) resource_calls: u64,
    pub(in crate::executable) unique_calls: u64,
    pub(in crate::executable) structural_calls: u64,
    pub(in crate::executable) heap_arguments: Vec<NativeValue>,
    pub(in crate::executable) heap_operation_attempts: u64,
    pub(in crate::executable) heap_operation_successes: u64,
    pub(in crate::executable) cleanup_failures: Vec<NativeCleanupFailure>,
    pub(in crate::executable) omitted_cleanup_failures: usize,
    pub(in crate::executable) maximum_cleanup_failures: usize,
    pub(in crate::executable) entry_rejected: bool,
    pub(in crate::executable) invalid_entry_accounting: Option<u64>,
    pub(in crate::executable) bookkeeping_allocation_failed: bool,
    pub(in crate::executable) metadata_invalid: bool,
}

impl<'a> IslandCallState<'a> {
    pub(in crate::executable) fn new(
        image: &'a InstallableImage,
        entry_mapping: &'a NativeEntryMapping,
        config: &NativeInvocationConfig,
        services: &'a mut dyn NativeIslandRuntimeServices,
    ) -> Result<Self, InvocationError> {
        let native_entries = try_entry_counts(image)?;
        let mut active_frames = Vec::new();
        active_frames
            .try_reserve_exact(config.max_active_frames.min(image.entries().len()))
            .map_err(|_| InvocationError::NativeBookkeepingAllocationFailed)?;
        let (native_stack_low, native_stack_high) =
            platform::native_stack_bounds().unwrap_or((0, 0));
        let (deadline_ms, status) = match config.wall_time {
            Some(duration) => {
                let now = crate::now_ms_monotonic();
                let delta = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
                (now.saturating_add(delta), 0)
            }
            None => (-1, 0),
        };
        Ok(Self {
            status,
            trap: 0,
            payload: 0,
            trap_site_present: 0,
            _scratch_integer_arguments: [0; 5],
            _scratch_float_arguments: [0; 2],
            poll_fuel_remaining: config.poll_fuel,
            deadline_ms,
            poll_count: 0,
            native_entries,
            entry_mapping,
            image,
            services,
            active_frames,
            maximum_active_frames: config.max_active_frames,
            maximum_active_values: config.max_active_values,
            maximum_native_stack_bytes: config.max_native_stack_bytes,
            maximum_native_frame_bytes: config.max_native_frame_bytes,
            native_stack_low,
            native_stack_high,
            pending_reservation: None,
            reserved_native_stack_bytes: 0,
            peak_native_stack_bytes: 0,
            peak_active_depth: 0,
            active_value_homes: 0,
            peak_active_value_homes: 0,
            resource_calls: 0,
            unique_calls: 0,
            structural_calls: 0,
            heap_arguments: Vec::new(),
            heap_operation_attempts: 0,
            heap_operation_successes: 0,
            cleanup_failures: Vec::new(),
            omitted_cleanup_failures: 0,
            maximum_cleanup_failures: config.max_cleanup_failures,
            entry_rejected: false,
            invalid_entry_accounting: None,
            bookkeeping_allocation_failed: false,
            metadata_invalid: false,
        })
    }

    pub(in crate::executable) fn poll(&mut self) {
        if self.status != 0 {
            return;
        }
        self.poll_count = self.poll_count.saturating_add(1);
        if self.poll_fuel_remaining == 0 {
            self.status = 4;
            self.payload = 1;
            return;
        }
        self.poll_fuel_remaining -= 1;
        if self.deadline_ms >= 0 && crate::now_ms_monotonic() >= self.deadline_ms {
            self.status = 3;
        }
    }

    pub(in crate::executable) fn fail_bookkeeping_allocation(&mut self) {
        if let Some(reservation) = self.pending_reservation.take() {
            self.reserved_native_stack_bytes = self
                .reserved_native_stack_bytes
                .saturating_sub(reservation.frame_bytes);
            self.active_value_homes = self
                .active_value_homes
                .saturating_sub(reservation.value_homes);
        }
        self.bookkeeping_allocation_failed = true;
        self.status = 5;
    }

    pub(in crate::executable) fn invalidate_frame(&mut self) {
        if let Some(reservation) = self.pending_reservation.take() {
            self.reserved_native_stack_bytes = self
                .reserved_native_stack_bytes
                .saturating_sub(reservation.frame_bytes);
            self.active_value_homes = self
                .active_value_homes
                .saturating_sub(reservation.value_homes);
        }
        self.metadata_invalid = true;
        self.status = 5;
    }
}
