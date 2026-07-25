use super::*;

impl NativeCallState<'_> {
    pub(in crate::executable) fn collect_references(&mut self, argument: u64) -> u64 {
        if self.status != 0 {
            return argument;
        }
        self.roots.clear();
        self.root_addresses.clear();
        for frame_index in 0..self.active_depth {
            match self.materialize_frame_roots(frame_index) {
                Ok(()) => {}
                Err(MaterializeRootError::InvalidFrame) => {
                    self.invalidate_active_frame();
                    return argument;
                }
                Err(MaterializeRootError::Capacity) => {
                    self.status = 4;
                    self.payload = 3;
                    return argument;
                }
            }
        }
        let root_count = self.roots.len();
        if self.exact_root_counts.len() == MAX_COLLECTION_REPORTS
            || self.exact_root_counts.try_reserve(1).is_err()
        {
            self.status = 4;
            self.payload = 3;
            return argument;
        }
        self.collection_calls = self.collection_calls.saturating_add(1);
        self.maximum_roots = self.maximum_roots.max(root_count);
        self.exact_root_counts.push(root_count);
        match self.services.collect_references(&mut self.roots) {
            Ok(()) => {}
            Err(NativeServiceError::Trap) => {
                self.status = 1;
                self.trap = TrapCode::Explicit.as_u32();
                return argument;
            }
            Err(NativeServiceError::ResourceLimitExceeded) => {
                self.status = 4;
                self.payload = 4;
                return argument;
            }
            Err(NativeServiceError::HostFailure) => {
                self.status = 5;
                return argument;
            }
        }
        for (root, address) in self.roots.iter().zip(&self.root_addresses) {
            if root.reference_type != address.reference_type {
                self.invalidate_active_frame();
                return argument;
            }
            // SAFETY: materialize_frame_roots validated this exact aligned home
            // against retained image metadata and the live generated frame.
            unsafe { address.address.write(root.opaque_word) };
        }
        self.root_addresses
            .iter()
            .zip(&self.roots)
            .rev()
            .find(|(address, _)| {
                address.frame_index + 1 == self.active_depth
                    && address.original_word == argument
                    && address.reference_type == ReferenceType::Buf
            })
            .map_or(argument, |(_, root)| root.opaque_word)
    }
}
