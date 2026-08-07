use super::*;

impl IslandCallState<'_> {
    pub(in crate::executable) fn register_frame(&mut self, function_ordinal: u64, rbp: *mut u8) {
        if self.status != 0 {
            return;
        }
        let Some(reservation) = self.pending_reservation else {
            self.invalidate_frame();
            return;
        };
        if reservation.function_ordinal != function_ordinal
            || reservation.rbp != rbp
            || self
                .maximum_active_frames
                .is_some_and(|maximum| self.active_frames.len() >= maximum)
        {
            self.invalidate_frame();
            return;
        }
        let Ok(ordinal) = usize::try_from(function_ordinal) else {
            self.invalidate_frame();
            return;
        };
        if !record_native_entry(&mut self.native_entries, ordinal) {
            self.invalid_entry_accounting = self
                .image
                .entries()
                .get(ordinal)
                .map(|entry| entry.source_function().get())
                .or(Some(function_ordinal));
            self.status = 5;
            return;
        }
        if self.active_frames.len() == self.active_frames.capacity()
            && self.active_frames.try_reserve(1).is_err()
        {
            self.fail_bookkeeping_allocation();
            return;
        }
        self.poll();
        if self.status != 0 {
            self.pending_reservation = None;
            let Some(next_reserved) = self
                .reserved_native_stack_bytes
                .checked_sub(reservation.frame_bytes)
            else {
                self.invalidate_frame();
                return;
            };
            let Some(next_values) = self.active_value_homes.checked_sub(reservation.value_homes)
            else {
                self.invalidate_frame();
                return;
            };
            self.reserved_native_stack_bytes = next_reserved;
            self.active_value_homes = next_values;
            return;
        }
        self.pending_reservation = None;
        self.active_frames.push(IslandFrame {
            function_ordinal,
            rbp,
            reserved_bytes: reservation.frame_bytes,
            value_homes: reservation.value_homes,
        });
        self.peak_active_depth = self.peak_active_depth.max(self.active_frames.len());
    }

    pub(in crate::executable) fn unregister_frame(&mut self, function_ordinal: u64, rbp: *mut u8) {
        let Some(frame) = self.active_frames.last().copied() else {
            self.invalidate_frame();
            return;
        };
        if frame.function_ordinal != function_ordinal || frame.rbp != rbp {
            self.invalidate_frame();
            return;
        }
        let Some(next_bytes) = self
            .reserved_native_stack_bytes
            .checked_sub(frame.reserved_bytes)
        else {
            self.invalidate_frame();
            return;
        };
        let Some(next_values) = self.active_value_homes.checked_sub(frame.value_homes) else {
            self.invalidate_frame();
            return;
        };
        self.active_frames.pop();
        self.reserved_native_stack_bytes = next_bytes;
        self.active_value_homes = next_values;
    }
}
