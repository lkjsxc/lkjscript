use super::*;

impl NativeCallState<'_> {
    pub(in crate::executable) fn dispatch_heap_operation(&mut self, site_id: u32) {
        if self.status != 0 {
            return;
        }
        let Some(site) = self
            .image
            .heap_runtime_sites()
            .get(site_id as usize)
            .cloned()
        else {
            self.invalidate_active_frame();
            return;
        };
        if site.id() != site_id || site.safepoint() as usize >= self.image.safepoints().len() {
            self.invalidate_active_frame();
            return;
        }
        let Some(frame_index) = self.active_depth.checked_sub(1) else {
            self.invalidate_active_frame();
            return;
        };
        let frame = self.active_frames[frame_index];
        let Some(entry) = self.image.entries().get(frame.function_ordinal as usize) else {
            self.invalidate_active_frame();
            return;
        };
        if entry.function() != site.function() || frame.safepoint != site.safepoint() {
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
        self.roots.clear();
        self.root_addresses.clear();
        for active in 0..self.active_depth {
            match self.materialize_frame_roots(active) {
                Ok(()) => {}
                Err(MaterializeRootError::InvalidFrame) => {
                    self.invalidate_active_frame();
                    return;
                }
                Err(MaterializeRootError::Capacity) => {
                    self.status = 4;
                    self.payload = 3;
                    return;
                }
            }
        }
        let collected =
            self.services
                .prepare_heap_operation(&site, &self.heap_arguments, &mut self.roots);
        let collected = match collected {
            Ok(collected) => collected,
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
        for (root, address) in self.roots.iter().zip(&self.root_addresses) {
            if root.reference_type != address.reference_type {
                self.invalidate_active_frame();
                return;
            }
            // SAFETY: root addresses came only from validated active homes.
            unsafe { address.address.write(root.opaque_word) };
        }
        // A collecting service may move a live argument and rewrite its frame
        // home through the root set. Re-read every verified argument home only
        // after root writeback so heap_operation never receives stale words.
        if !self.materialize_heap_arguments(&site, &facts, frame) {
            self.invalidate_active_frame();
            return;
        }
        if collected {
            let count = self.roots.len();
            if self.exact_root_counts.len() == MAX_COLLECTION_REPORTS
                || self.exact_root_counts.try_reserve(1).is_err()
            {
                self.status = 4;
                self.payload = 3;
                return;
            }
            self.collection_calls = self.collection_calls.saturating_add(1);
            self.maximum_roots = self.maximum_roots.max(count);
            self.exact_root_counts.push(count);
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
        if site.descriptor().store() != StoreClass::None {
            self.barrier_count = self.barrier_count.saturating_add(1);
        }
    }

    pub(in crate::executable) fn materialize_heap_arguments(
        &mut self,
        site: &HeapRuntimeSite,
        facts: &lkjscript_native::FrameFacts,
        frame: ActiveFrame,
    ) -> bool {
        self.heap_arguments.clear();
        for home in site.arguments() {
            if !facts.homes().contains(home) {
                return false;
            }
            // SAFETY: retained image integrity and the active descriptor bind
            // this aligned home to the currently registered generated frame.
            let address = unsafe {
                frame
                    .rbp
                    .offset(home.rbp_displacement() as isize)
                    .cast::<u64>()
            };
            // SAFETY: each verified argument home is initialized at this site.
            let word = unsafe { address.read() };
            let value = match home.value_type() {
                ValueType::I64 => NativeValue::I64(word as i64),
                ValueType::F64 => NativeValue::F64Bits(word),
                ValueType::Bool if word <= 1 => NativeValue::Bool(word == 1),
                ValueType::Bool => return false,
                ValueType::Unit if word == 0 => NativeValue::Unit,
                ValueType::Unit => return false,
                ValueType::Capability(_)
                | ValueType::Resource(_)
                | ValueType::Unique(_)
                | ValueType::Loan(_) => return false,
                ValueType::Reference(reference_type) => {
                    NativeValue::Reference(NativeReference::new(reference_type, word))
                }
            };
            self.heap_arguments.push(value);
        }
        true
    }
}
