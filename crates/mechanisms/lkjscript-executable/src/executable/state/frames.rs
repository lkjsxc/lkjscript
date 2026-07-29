use super::*;

impl NativeCallState<'_> {
    pub(in crate::executable) fn register_frame(&mut self, function_ordinal: u32, rbp: *mut u8) {
        if self.status != 0 {
            return;
        }
        let Some(reservation) = self.pending_reservation else {
            self.invalidate_active_frame();
            return;
        };
        if reservation.function_ordinal != function_ordinal
            || reservation.rbp != rbp
            || self.active_depth >= self.maximum_active_frames
        {
            self.invalidate_active_frame();
            return;
        }
        let Some(source_function) = self
            .image
            .entries()
            .get(function_ordinal as usize)
            .map(|entry| entry.source_function().get() as usize)
        else {
            self.invalidate_active_frame();
            return;
        };
        let Some(entries) = self.native_entries.get_mut(source_function) else {
            self.invalidate_active_frame();
            return;
        };
        *entries = entries.saturating_add(1);
        self.poll();
        if self.status != 0 {
            if let Some(reservation) = self.pending_reservation.take() {
                self.reserved_native_stack_bytes = self
                    .reserved_native_stack_bytes
                    .saturating_sub(reservation.frame_bytes);
                self.active_value_homes = self
                    .active_value_homes
                    .saturating_sub(reservation.value_homes);
            }
            return;
        }
        self.pending_reservation = None;
        self.active_frames[self.active_depth] = ActiveFrame {
            function_ordinal,
            rbp,
            safepoint: INVALID_SAFEPOINT,
            reserved_bytes: reservation.frame_bytes,
            value_homes: reservation.value_homes,
        };
        self.active_depth += 1;
        self.peak_active_depth = self.peak_active_depth.max(self.active_depth);
    }

    pub(in crate::executable) fn publish_safepoint(&mut self, safepoint: u32) {
        if self.status != 0 {
            return;
        }
        let Some(frame) = self
            .active_frames
            .get_mut(self.active_depth.saturating_sub(1))
        else {
            self.invalidate_active_frame();
            return;
        };
        let Some(entry) = self.image.entries().get(frame.function_ordinal as usize) else {
            self.invalidate_active_frame();
            return;
        };
        let valid = self
            .image
            .safepoints()
            .get(safepoint as usize)
            .is_some_and(|map| map.id() == safepoint && map.function() == entry.function());
        if !valid {
            self.invalidate_active_frame();
            return;
        }
        frame.safepoint = safepoint;
    }

    pub(in crate::executable) fn unregister_frame(&mut self, function_ordinal: u32, rbp: *mut u8) {
        let Some(index) = self.active_depth.checked_sub(1) else {
            self.invalidate_active_frame();
            return;
        };
        let frame = self.active_frames[index];
        if frame.function_ordinal != function_ordinal || frame.rbp != rbp {
            self.invalidate_active_frame();
            return;
        }
        let Some(next_reserved_bytes) = self
            .reserved_native_stack_bytes
            .checked_sub(frame.reserved_bytes)
        else {
            self.invalidate_active_frame();
            return;
        };
        let Some(next_active_values) = self.active_value_homes.checked_sub(frame.value_homes)
        else {
            self.invalidate_active_frame();
            return;
        };
        self.active_frames[index] = EMPTY_ACTIVE_FRAME;
        self.active_depth = index;
        self.reserved_native_stack_bytes = next_reserved_bytes;
        self.active_value_homes = next_active_values;
    }
}
