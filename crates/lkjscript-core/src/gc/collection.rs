use super::{allocation::estimated_object_bytes, GcHeap, GcStats};
use crate::Value;

impl GcHeap {
    /// Mark/sweep from exact roots. Invalid/category checking belongs to the
    /// typed VM/JIT adapters; unknown heap words simply do not retain storage.
    pub fn collect(&mut self, roots: &[Value]) {
        let count = self.objs.len();
        let mut marked = vec![false; count];
        let mut pending = roots.to_vec();
        while let Some(value) = pending.pop() {
            if let Some(index) = value.as_heap() {
                let index = index as usize;
                if index < count && !marked[index] {
                    marked[index] = true;
                    if let Some(Some(object)) = self.objs.get(index) {
                        object.trace(&mut |child| pending.push(child));
                    }
                }
            }
        }
        for (index, slot) in self.objs.iter_mut().enumerate() {
            if let Some(object) = slot.as_ref().filter(|_| !marked[index]) {
                self.stats.live_heap_bytes = self
                    .stats
                    .live_heap_bytes
                    .saturating_sub(estimated_object_bytes(object));
                *slot = None;
                self.layout_tags[index] = None;
            }
        }
        self.allocs_since_gc = 0;
        self.stats.collections = self.stats.collections.saturating_add(1);
    }

    #[must_use]
    pub const fn collect_before_allocation(&self) -> bool {
        self.config.collect_before_every_allocation
            || self.allocs_since_gc >= self.config.collect_after_allocations
    }

    #[must_use]
    pub const fn needs_collect(&self) -> bool {
        self.collect_before_allocation()
    }

    /// Current deterministic object-size estimate, not allocator/RSS bytes.
    #[must_use]
    pub const fn heap_bytes(&self) -> usize {
        self.stats.live_heap_bytes
    }

    #[must_use]
    pub const fn total_allocations(&self) -> u64 {
        self.stats.allocations
    }

    /// Cumulative deterministic object-size estimate, not allocator/RSS bytes.
    #[must_use]
    pub const fn total_allocated_bytes(&self) -> u64 {
        self.stats.allocated_bytes
    }

    #[must_use]
    pub const fn collections(&self) -> u64 {
        self.stats.collections
    }

    /// Peak deterministic object-size estimate, not allocator/RSS bytes.
    #[must_use]
    pub const fn peak_live_heap_bytes(&self) -> usize {
        self.stats.peak_live_heap_bytes
    }

    #[must_use]
    pub const fn stats(&self) -> GcStats {
        self.stats
    }
}
