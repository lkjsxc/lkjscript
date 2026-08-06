#![allow(unsafe_code)]

use super::*;

impl IslandCallState<'_> {
    pub(in crate::executable) fn dispatch_runtime_value_operation(&mut self, site_id: u32) {
        if self.status != 0 {
            return;
        }
        let Some(site) = self
            .image
            .heap_runtime_sites()
            .get(site_id as usize)
            .cloned()
        else {
            self.invalidate_frame();
            return;
        };
        if site.id() != site_id {
            self.invalidate_frame();
            return;
        }
        let Some(frame_index) = self.active_depth.checked_sub(1) else {
            self.invalidate_frame();
            return;
        };
        let frame = self.active_frames[frame_index];
        let Some(entry) = self.image.entries().get(frame.function_ordinal as usize) else {
            self.invalidate_frame();
            return;
        };
        if entry.function() != site.function() {
            self.invalidate_frame();
            return;
        }
        let Some(facts) = self
            .image
            .frames()
            .iter()
            .find(|facts| facts.function() == site.function())
            .cloned()
        else {
            self.invalidate_frame();
            return;
        };
        if !self.materialize_heap_arguments(&site, &facts, frame) {
            self.invalidate_frame();
            return;
        }
        self.heap_operation_attempts = self.heap_operation_attempts.saturating_add(1);
        let result = match self.services.heap_operation(&site, &self.heap_arguments) {
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
            self.invalidate_frame();
            return;
        }
        let Some(word) = native_value_word(result, site.descriptor().result_type()) else {
            self.invalidate_frame();
            return;
        };
        let result_home = site.result();
        if !facts.homes().contains(&result_home) {
            self.invalidate_frame();
            return;
        }
        // SAFETY: retained image integrity binds this initialized result home to
        // the current registered generated frame.
        unsafe {
            frame
                .rbp
                .offset(result_home.rbp_displacement() as isize)
                .cast::<u64>()
                .write(word)
        };
        self.heap_operation_successes = self.heap_operation_successes.saturating_add(1);
    }

    fn materialize_heap_arguments(
        &mut self,
        site: &HeapRuntimeSite,
        facts: &lkjscript_native::FrameFacts,
        frame: IslandFrame,
    ) -> bool {
        self.heap_arguments.clear();
        for home in site.arguments() {
            if !facts.homes().contains(home) {
                return false;
            }
            // SAFETY: image integrity binds this aligned initialized home to the
            // current registered generated frame.
            let word = unsafe {
                frame
                    .rbp
                    .offset(home.rbp_displacement() as isize)
                    .cast::<u64>()
                    .read()
            };
            let value = match home.value_type() {
                ValueType::I64 => NativeValue::I64(word as i64),
                ValueType::F64 => NativeValue::F64Bits(word),
                ValueType::Bool if word <= 1 => NativeValue::Bool(word == 1),
                ValueType::Unit if word == 0 => NativeValue::Unit,
                ValueType::Reference(value_type) => {
                    NativeValue::Reference(NativeReference::new(value_type, word))
                }
                ValueType::StaticString(value_type) if word != 0 => {
                    NativeValue::StaticString(NativeStaticString::new(value_type, word))
                }
                ValueType::StructuralOwner(value_type) if word != 0 => {
                    NativeValue::StructuralOwner(NativeStructuralOwner::new(value_type, word))
                }
                _ => return false,
            };
            self.heap_arguments.push(value);
        }
        true
    }
}
