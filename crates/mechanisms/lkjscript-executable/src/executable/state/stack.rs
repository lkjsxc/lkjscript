use super::*;

impl NativeCallState<'_> {
    pub(in crate::executable) fn reserve_frame(
        &mut self,
        function_ordinal: u64,
        frame_bytes: u64,
        rbp: *mut u8,
    ) {
        if self.status != 0 {
            return;
        }
        if self.active_depth >= self.maximum_active_frames {
            self.status = 4;
            self.payload = 2;
            return;
        }
        if self.pending_reservation.is_some() {
            self.invalidate_active_frame();
            return;
        }
        let Ok(function_index) = usize::try_from(function_ordinal) else {
            self.invalidate_active_frame();
            return;
        };
        let Some(entry) = self.image.entries().get(function_index) else {
            self.invalidate_active_frame();
            return;
        };
        let Some((descriptor_bytes, value_homes)) = self
            .image
            .frames()
            .iter()
            .find(|frame| frame.function() == entry.function())
            .map(|frame| (frame.frame_bytes(), frame.homes().len()))
        else {
            self.invalidate_active_frame();
            return;
        };
        let Ok(frame_bytes) = usize::try_from(frame_bytes) else {
            self.invalidate_active_frame();
            return;
        };
        if usize::try_from(descriptor_bytes).ok() != Some(frame_bytes)
            || rbp.is_null()
            || !(rbp as usize).is_multiple_of(16)
        {
            self.invalidate_active_frame();
            return;
        }
        let Some(next_active_values) = self.active_value_homes.checked_add(value_homes) else {
            self.status = 4;
            self.payload = 6;
            return;
        };
        if next_active_values > self.maximum_active_values {
            self.status = 4;
            self.payload = 6;
            return;
        }
        let Some(next_reserved_bytes) = self.reserved_native_stack_bytes.checked_add(frame_bytes)
        else {
            self.status = 4;
            self.payload = 5;
            return;
        };
        if frame_bytes > self.maximum_native_frame_bytes
            || next_reserved_bytes > self.maximum_native_stack_bytes
            || !platform::native_stack_reservation_fits(
                rbp,
                frame_bytes,
                NATIVE_STACK_GUARD_BYTES,
                self.native_stack_low,
                self.native_stack_high,
            )
        {
            self.status = 4;
            self.payload = 5;
            return;
        }
        self.pending_reservation = Some(PendingFrameReservation {
            function_ordinal,
            rbp,
            frame_bytes,
            value_homes,
        });
        self.reserved_native_stack_bytes = next_reserved_bytes;
        self.active_value_homes = next_active_values;
        self.peak_active_value_homes = self.peak_active_value_homes.max(next_active_values);
        self.peak_native_stack_bytes = self
            .peak_native_stack_bytes
            .max(self.reserved_native_stack_bytes);
    }
}
