use super::*;

impl<'a> NativeCallState<'a> {
    pub(in crate::executable) fn new(
        image: &'a InstallableImage,
        config: &NativeInvocationConfig,
        services: &'a mut dyn NativeRuntimeServices,
    ) -> Result<Self, InvocationError> {
        let maximum_active_frames = config.max_active_frames.min(MAX_ACTIVE_FRAMES);
        let mut heap_arguments = Vec::new();
        heap_arguments
            .try_reserve_exact(16)
            .map_err(|_| InvocationError::RuntimeValueCapacityExceeded)?;
        // One generated invocation cannot migrate threads. Cache the current
        // thread's fixed stack bounds once instead of repeating pthread
        // attribute queries at every generated function entry.
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
            payload: -1,
            _scratch_integer_arguments: [0; 2],
            _scratch_float_arguments: [0; 2],
            poll_fuel_remaining: config.poll_fuel,
            deadline_ms,
            poll_count: 0,
            native_entries: [0; MAX_NATIVE_ENTRY_COUNTS],
            image,
            services,
            active_frames: [EMPTY_ACTIVE_FRAME; MAX_ACTIVE_FRAMES],
            active_depth: 0,
            maximum_active_frames,
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
            heap_arguments,
            heap_operation_attempts: 0,
            heap_operation_successes: 0,
            metadata_invalid: false,
        })
    }

    pub(in crate::executable) fn invalidate_active_frame(&mut self) {
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
}
