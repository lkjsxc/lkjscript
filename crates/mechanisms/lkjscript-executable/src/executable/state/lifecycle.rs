use super::*;

impl<'a> NativeCallState<'a> {
    pub(in crate::executable) fn new(
        image: &'a InstallableImage,
        entry_mapping: &'a NativeEntryMapping,
        config: &NativeInvocationConfig,
        deadline_ms: i64,
        native_stack_bounds: Option<platform::NativeStackBounds>,
        services: &'a mut dyn NativeRuntimeServices,
    ) -> Result<Self, PreEntryError> {
        let native_entries = try_entry_counts(image)?;
        let mut active_frames = Vec::new();
        active_frames
            .try_reserve_exact(config.max_active_frames.unwrap_or(image.entries().len()))
            .map_err(|_| PreEntryError::BookkeepingAllocationFailed)?;
        let mut heap_arguments = Vec::new();
        let maximum_heap_arguments = image
            .heap_runtime_sites()
            .iter()
            .map(|site| site.arguments().len())
            .max()
            .unwrap_or(0);
        heap_arguments
            .try_reserve_exact(maximum_heap_arguments)
            .map_err(|_| PreEntryError::BookkeepingAllocationFailed)?;
        Ok(Self {
            status: 0,
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
            native_stack_requirement: config.native_stack_requirement,
            native_stack_bounds,
            native_stack_error: None,
            pending_reservation: None,
            reserved_native_stack_bytes: 0,
            peak_native_stack_bytes: 0,
            peak_active_depth: 0,
            active_value_homes: 0,
            peak_active_value_homes: 0,
            heap_arguments,
            heap_operation_attempts: 0,
            heap_operation_successes: 0,
            invalid_entry_accounting: None,
            bookkeeping_allocation_failed: false,
            metadata_invalid: false,
            entry_started: false,
        })
    }

    pub(in crate::executable) fn decline_native_stack(&mut self, error: NativeStackError) {
        self.native_stack_error = Some(error);
        self.status = 6;
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
        if let Some(fuel) = &mut self.poll_fuel_remaining {
            if *fuel == 0 {
                self.status = 4;
                self.payload = 1;
                return;
            }
            *fuel -= 1;
        }
        if self.deadline_ms >= 0 && crate::now_ms_monotonic() >= self.deadline_ms {
            self.status = 3;
        }
    }
}
