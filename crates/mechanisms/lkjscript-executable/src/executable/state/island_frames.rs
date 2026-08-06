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
            || self.active_depth >= self.maximum_active_frames
        {
            self.invalidate_frame();
            return;
        }
        let Some(source) = usize::try_from(function_ordinal)
            .ok()
            .and_then(|function| self.image.entries().get(function))
            .and_then(|entry| usize::try_from(entry.source_function().get()).ok())
        else {
            self.invalidate_frame();
            return;
        };
        let Some(next_entries) = self
            .native_entries
            .get(source)
            .and_then(|entries| entries.checked_add(1))
        else {
            self.invalidate_frame();
            return;
        };
        self.native_entries[source] = next_entries;
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
        self.active_frames[self.active_depth] = IslandFrame {
            function_ordinal: Some(function_ordinal),
            rbp,
            reserved_bytes: reservation.frame_bytes,
            value_homes: reservation.value_homes,
        };
        self.active_depth += 1;
        self.peak_active_depth = self.peak_active_depth.max(self.active_depth);
    }

    pub(in crate::executable) fn unregister_frame(&mut self, function_ordinal: u64, rbp: *mut u8) {
        let Some(index) = self.active_depth.checked_sub(1) else {
            self.invalidate_frame();
            return;
        };
        let frame = self.active_frames[index];
        if frame.function_ordinal != Some(function_ordinal) || frame.rbp != rbp {
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
        self.active_frames[index] = EMPTY_ISLAND_FRAME;
        self.active_depth = index;
        self.reserved_native_stack_bytes = next_bytes;
        self.active_value_homes = next_values;
    }
}
