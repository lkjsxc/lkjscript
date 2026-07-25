use crate::HeapObj;

/// Bounded heap policy. Collection pressure is deterministic and may be forced
/// before every allocation for exact-root stress testing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GcConfig {
    pub max_allocations: u64,
    pub max_heap_bytes: usize,
    pub collect_after_allocations: u32,
    pub collect_before_every_allocation: bool,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            max_allocations: u64::MAX,
            max_heap_bytes: usize::MAX,
            collect_after_allocations: 1_024,
            collect_before_every_allocation: false,
        }
    }
}

/// Exact retained counters for one heap session.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GcStats {
    pub allocations: u64,
    /// Cumulative deterministic object-size estimate, not allocator/RSS bytes.
    pub allocated_bytes: u64,
    pub collections: u64,
    /// Current deterministic object-size estimate, not allocator/RSS bytes.
    pub live_heap_bytes: usize,
    /// Peak deterministic object-size estimate, not allocator/RSS bytes.
    pub peak_live_heap_bytes: usize,
}

/// A fully initialized object rejected by a configured heap boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GcLimit {
    Allocations,
    HeapBytes,
}

/// Stable-index, non-moving mark/sweep heap. An object is inserted into the
/// traced object table only after its `HeapObj` value is completely built.
#[derive(Debug)]
pub struct GcHeap {
    pub(super) objs: Vec<Option<HeapObj>>,
    pub(super) layout_tags: Vec<Option<u64>>,
    pub(super) allocs_since_gc: u32,
    pub(super) config: GcConfig,
    pub(super) stats: GcStats,
}

impl Default for GcHeap {
    fn default() -> Self {
        Self::new(GcConfig::default())
    }
}

impl GcHeap {
    #[must_use]
    pub const fn new(config: GcConfig) -> Self {
        Self {
            objs: Vec::new(),
            layout_tags: Vec::new(),
            allocs_since_gc: 0,
            config,
            stats: GcStats {
                allocations: 0,
                allocated_bytes: 0,
                collections: 0,
                live_heap_bytes: 0,
                peak_live_heap_bytes: 0,
            },
        }
    }

    #[must_use]
    pub const fn config(&self) -> GcConfig {
        self.config
    }

    pub fn set_config(&mut self, config: GcConfig) {
        self.config = config;
    }
}
