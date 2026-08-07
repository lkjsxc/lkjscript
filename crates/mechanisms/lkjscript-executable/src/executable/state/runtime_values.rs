#![allow(unsafe_code)]

use super::*;

mod arguments;

impl NativeCallState<'_> {
    pub(in crate::executable) fn dispatch_runtime_value_operation(&mut self, site_id: u64) {
        if self.status != 0 {
            return;
        }
        let Ok(site_index) = usize::try_from(site_id) else {
            self.invalidate_active_frame();
            return;
        };
        let Some(site) = self.image.heap_runtime_sites().get(site_index).cloned() else {
            self.invalidate_active_frame();
            return;
        };
        if site.id() != site_id {
            self.invalidate_active_frame();
            return;
        }
        let Some(frame) = self.active_frames.last().copied() else {
            self.invalidate_active_frame();
            return;
        };
        let Some(entry) = usize::try_from(frame.function_ordinal)
            .ok()
            .and_then(|function| self.image.entries().get(function))
        else {
            self.invalidate_active_frame();
            return;
        };
        if entry.function() != site.function() {
            self.invalidate_active_frame();
            return;
        }
        let Some(facts) = self
            .image
            .frames()
            .iter()
            .find(|facts| facts.function() == site.function())
            .cloned()
        else {
            self.invalidate_active_frame();
            return;
        };
        if !self.materialize_heap_arguments(&site, &facts, frame) {
            self.invalidate_active_frame();
            return;
        }
        self.heap_operation_attempts = self.heap_operation_attempts.saturating_add(1);
        let result = self.services.heap_operation(&site, &self.heap_arguments);
        let result = match result {
            Ok(result) => result,
            Err(NativeServiceError::Trap) => {
                self.status = 1;
                self.trap = TrapCode::Explicit.as_u32();
                return;
            }
            Err(NativeServiceError::ResourceLimitExceeded) => {
                self.status = 4;
                self.payload = 4;
                return;
            }
            Err(NativeServiceError::HostFailure) => {
                self.status = 5;
                return;
            }
        };
        if result.value_type() != site.descriptor().result_type() {
            self.invalidate_active_frame();
            return;
        }
        let Some(word) = native_value_word(result, site.descriptor().result_type()) else {
            self.invalidate_active_frame();
            return;
        };
        let result_home = site.result();
        if !facts.homes().contains(&result_home) {
            self.invalidate_active_frame();
            return;
        }
        // SAFETY: retained site integrity binds this exact initialized result
        // home to the current active generated frame.
        unsafe {
            frame
                .rbp
                .offset(result_home.rbp_displacement() as isize)
                .cast::<u64>()
                .write(word)
        };
        self.heap_operation_successes = self.heap_operation_successes.saturating_add(1);
    }
}
