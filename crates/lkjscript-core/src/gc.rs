//! Pure session-owned stable-index mark/sweep heap shared by VM and JIT.

use crate::{Error, HeapObj, OwnedValue, Result, Value};

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
    pub allocated_bytes: u64,
    pub collections: u64,
    pub live_heap_bytes: usize,
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
    objs: Vec<Option<HeapObj>>,
    free: Vec<u32>,
    layout_tags: Vec<Option<u64>>,
    allocs_since_gc: u32,
    config: GcConfig,
    stats: GcStats,
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
            free: Vec::new(),
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

    /// Compatibility allocation for the VM's existing instruction-boundary
    /// accounting. Bounded generated execution uses `try_alloc`.
    pub fn alloc(&mut self, object: HeapObj) -> Value {
        self.publish(object)
    }

    /// Publish one completely initialized object after checking exact aggregate
    /// allocation and live-byte bounds. Callers collect first when
    /// `collect_before_allocation` is true.
    pub fn try_alloc(&mut self, object: HeapObj) -> std::result::Result<Value, GcLimit> {
        if self.stats.allocations >= self.config.max_allocations {
            return Err(GcLimit::Allocations);
        }
        let bytes = estimated_object_bytes(&object);
        let next = self
            .stats
            .live_heap_bytes
            .checked_add(bytes)
            .ok_or(GcLimit::HeapBytes)?;
        if next > self.config.max_heap_bytes {
            return Err(GcLimit::HeapBytes);
        }
        Ok(self.publish_with_layout(object, None))
    }

    /// Bounded typed publication used by generated runtime adapters. The
    /// opaque layout tag is checked on every later typed handle conversion.
    pub fn try_alloc_with_layout(
        &mut self,
        object: HeapObj,
        layout: u64,
    ) -> std::result::Result<Value, GcLimit> {
        if self.stats.allocations >= self.config.max_allocations {
            return Err(GcLimit::Allocations);
        }
        let bytes = estimated_object_bytes(&object);
        let next = self
            .stats
            .live_heap_bytes
            .checked_add(bytes)
            .ok_or(GcLimit::HeapBytes)?;
        if next > self.config.max_heap_bytes {
            return Err(GcLimit::HeapBytes);
        }
        Ok(self.publish_with_layout(object, Some(layout)))
    }

    fn publish(&mut self, object: HeapObj) -> Value {
        self.publish_with_layout(object, None)
    }

    fn publish_with_layout(&mut self, object: HeapObj, layout: Option<u64>) -> Value {
        let bytes = estimated_object_bytes(&object);
        self.allocs_since_gc = self.allocs_since_gc.saturating_add(1);
        self.stats.allocations = self.stats.allocations.saturating_add(1);
        self.stats.allocated_bytes = self
            .stats
            .allocated_bytes
            .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
        self.stats.live_heap_bytes = self.stats.live_heap_bytes.saturating_add(bytes);
        self.stats.peak_live_heap_bytes = self
            .stats
            .peak_live_heap_bytes
            .max(self.stats.live_heap_bytes);
        if let Some(index) = self.free.pop() {
            self.objs[index as usize] = Some(object);
            self.layout_tags[index as usize] = layout;
            return Value::from_heap(index);
        }
        let index = u32::try_from(self.objs.len()).unwrap_or(u32::MAX);
        self.objs.push(Some(object));
        self.layout_tags.push(layout);
        Value::from_heap(index)
    }

    pub fn get(&self, value: Value) -> Result<&HeapObj> {
        let index = value
            .as_heap()
            .ok_or_else(|| Error::msg("expected heap value"))? as usize;
        self.objs
            .get(index)
            .and_then(Option::as_ref)
            .ok_or_else(|| Error::msg("bad heap index"))
    }

    pub fn get_mut(&mut self, value: Value) -> Result<&mut HeapObj> {
        let index = value
            .as_heap()
            .ok_or_else(|| Error::msg("expected heap value"))? as usize;
        self.objs
            .get_mut(index)
            .and_then(Option::as_mut)
            .ok_or_else(|| Error::msg("bad heap index"))
    }

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
        self.free.clear();
        for (index, slot) in self.objs.iter_mut().enumerate() {
            if let Some(object) = slot.as_ref().filter(|_| !marked[index]) {
                self.stats.live_heap_bytes = self
                    .stats
                    .live_heap_bytes
                    .saturating_sub(estimated_object_bytes(object));
                *slot = None;
                self.layout_tags[index] = None;
                self.free.push(index as u32);
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

    #[must_use]
    pub const fn heap_bytes(&self) -> usize {
        self.stats.live_heap_bytes
    }

    #[must_use]
    pub const fn total_allocations(&self) -> u64 {
        self.stats.allocations
    }

    #[must_use]
    pub const fn total_allocated_bytes(&self) -> u64 {
        self.stats.allocated_bytes
    }

    #[must_use]
    pub const fn collections(&self) -> u64 {
        self.stats.collections
    }

    #[must_use]
    pub const fn peak_live_heap_bytes(&self) -> usize {
        self.stats.peak_live_heap_bytes
    }

    #[must_use]
    pub const fn stats(&self) -> GcStats {
        self.stats
    }

    /// Validate that a stable heap handle still names a published object.
    pub fn validate_handle(&self, value: Value) -> Result<()> {
        self.get(value).map(|_| ())
    }

    #[must_use]
    pub fn layout_of(&self, value: Value) -> Option<u64> {
        let index = value.as_heap()? as usize;
        self.objs.get(index)?.as_ref()?;
        self.layout_tags.get(index).copied().flatten()
    }

    pub fn into_owned(mut self, root: Value) -> Result<OwnedValue> {
        self.collect(&[root]);
        OwnedValue::from_vm_snapshot(root, self.objs)
    }

    /// Build an owned reachable snapshot without consuming the session heap.
    pub fn snapshot(&self, root: Value) -> Result<OwnedValue> {
        OwnedValue::from_vm_snapshot(root, self.objs.clone())
    }
}

fn estimated_object_bytes(object: &HeapObj) -> usize {
    let base = std::mem::size_of::<HeapObj>();
    let dynamic = match object {
        HeapObj::Str(text) | HeapObj::Symbol(text) => text.capacity(),
        HeapObj::Pair { .. }
        | HeapObj::Int(_)
        | HeapObj::Float(_)
        | HeapObj::Builtin(_)
        | HeapObj::ResultOk(_)
        | HeapObj::ResultErr(_)
        | HeapObj::OptionSome(_) => 0,
        HeapObj::Closure { captures, .. } => captures
            .capacity()
            .saturating_mul(std::mem::size_of::<Value>()),
        HeapObj::Buf(bytes) => bytes.capacity(),
        HeapObj::Product { fields, .. } => fields
            .capacity()
            .saturating_mul(std::mem::size_of::<Value>()),
    };
    base.saturating_add(dynamic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_preserves_nested_graph_and_reports_exact_counters() {
        let mut heap = GcHeap::new(GcConfig {
            collect_before_every_allocation: true,
            ..GcConfig::default()
        });
        let payload = heap.alloc(HeapObj::Str("nested".into()));
        let list = heap.alloc(HeapObj::Pair {
            car: payload,
            cdr: Value::EMPTY_LIST,
        });
        let result = heap.alloc(HeapObj::ResultOk(list));
        let option = heap.alloc(HeapObj::OptionSome(result));
        let product = heap.alloc(HeapObj::Product {
            product: crate::ProductId::new(0),
            fields: vec![option],
        });
        heap.collect(&[product]);
        for value in [payload, list, result, option, product] {
            assert!(heap.get(value).is_ok());
        }
        assert_eq!(heap.total_allocations(), 5);
        assert_eq!(heap.collections(), 1);
        assert!(heap.total_allocated_bytes() >= heap.heap_bytes() as u64);
        assert!(heap.peak_live_heap_bytes() >= heap.heap_bytes());
    }

    #[test]
    fn bounded_publication_rejects_without_partial_object() {
        let mut heap = GcHeap::new(GcConfig {
            max_allocations: 1,
            ..GcConfig::default()
        });
        assert!(heap.try_alloc(HeapObj::Str("one".into())).is_ok());
        assert_eq!(
            heap.try_alloc(HeapObj::Str("two".into())),
            Err(GcLimit::Allocations)
        );
        assert_eq!(heap.total_allocations(), 1);
    }
}
