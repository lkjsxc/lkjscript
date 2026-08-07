use super::*;

impl IslandCallState<'_> {
    pub(in crate::executable) fn reserve_frame(
        &mut self,
        function_ordinal: u64,
        frame_bytes: u64,
        rbp: *mut u8,
    ) {
        if self.status != 0 {
            return;
        }
        if self
            .maximum_active_frames
            .is_some_and(|maximum| self.active_frames.len() >= maximum)
        {
            self.status = 4;
            self.payload = 2;
            return;
        }
        if self.pending_reservation.is_some() {
            self.invalidate_frame();
            return;
        }
        let Ok(function_index) = usize::try_from(function_ordinal) else {
            self.invalidate_frame();
            return;
        };
        let Some(entry) = self.image.entries().get(function_index) else {
            self.invalidate_frame();
            return;
        };
        let Some((descriptor_bytes, value_homes)) = self
            .image
            .frames()
            .iter()
            .find(|frame| frame.function() == entry.function())
            .map(|frame| (frame.frame_bytes(), frame.homes().len()))
        else {
            self.invalidate_frame();
            return;
        };
        let Ok(frame_bytes) = usize::try_from(frame_bytes) else {
            self.decline_native_stack(NativeStackError::FrameArithmeticOverflow);
            return;
        };
        if usize::try_from(descriptor_bytes).ok() != Some(frame_bytes)
            || rbp.is_null()
            || !(rbp as usize).is_multiple_of(16)
        {
            self.invalidate_frame();
            return;
        }
        let Some(next_values) = self.active_value_homes.checked_add(value_homes) else {
            if self.maximum_active_values.is_some() {
                self.status = 4;
                self.payload = 6;
            } else {
                self.decline_native_stack(NativeStackError::FrameArithmeticOverflow);
            }
            return;
        };
        if self
            .maximum_active_values
            .is_some_and(|maximum| next_values > maximum)
        {
            self.status = 4;
            self.payload = 6;
            return;
        }
        let Some(next_bytes) = self.reserved_native_stack_bytes.checked_add(frame_bytes) else {
            self.decline_native_stack(NativeStackError::FrameArithmeticOverflow);
            return;
        };
        let Some(bounds) = self.native_stack_bounds else {
            self.decline_native_stack(NativeStackError::ThreadExtentUnavailable);
            return;
        };
        if self.active_frames.is_empty() {
            if let Some(required_bytes) = self.native_stack_requirement {
                if let Err(boundary) =
                    platform::native_stack_reservation_fits(rbp, required_bytes, bounds)
                {
                    self.decline_native_stack(boundary);
                    return;
                }
            }
        }
        if let Err(boundary) = platform::native_stack_reservation_fits(rbp, frame_bytes, bounds) {
            self.decline_native_stack(boundary);
            return;
        }
        self.pending_reservation = Some(IslandFrameReservation {
            function_ordinal,
            rbp,
            frame_bytes,
            value_homes,
        });
        self.reserved_native_stack_bytes = next_bytes;
        self.active_value_homes = next_values;
        self.peak_active_value_homes = self.peak_active_value_homes.max(next_values);
        self.peak_native_stack_bytes = self.peak_native_stack_bytes.max(next_bytes);
    }
}
