//! Pure session-owned stable-index mark/sweep heap shared by VM and JIT.

use crate::{Error, HeapObj, OwnedValue, ResourceLimitKind, Result, Value};

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
    objs: Vec<Option<HeapObj>>,
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

    /// Compatibility allocation for VM and host helpers. Publication checks
    /// configured limits and stable-handle exhaustion before assigning an ID.
    pub fn alloc(&mut self, object: HeapObj) -> Result<Value> {
        self.try_alloc(object).map_err(gc_limit_error)
    }

    /// Publish one completely initialized object after checking exact aggregate
    /// allocation and live-byte bounds. Callers collect first when
    /// `collect_before_allocation` is true.
    pub fn try_alloc(&mut self, object: HeapObj) -> std::result::Result<Value, GcLimit> {
        if self.stats.allocations >= self.config.max_allocations
            || !stable_index_available(self.objs.len())
        {
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
        self.publish_with_layout(object, None)
    }

    /// Bounded typed publication used by generated runtime adapters. The
    /// opaque layout tag is checked on every later typed handle conversion.
    pub fn try_alloc_with_layout(
        &mut self,
        object: HeapObj,
        layout: u64,
    ) -> std::result::Result<Value, GcLimit> {
        if self.stats.allocations >= self.config.max_allocations
            || !stable_index_available(self.objs.len())
        {
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
        self.publish_with_layout(object, Some(layout))
    }

    fn publish_with_layout(
        &mut self,
        object: HeapObj,
        layout: Option<u64>,
    ) -> std::result::Result<Value, GcLimit> {
        let index = u32::try_from(self.objs.len()).map_err(|_| GcLimit::Allocations)?;
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
        // Handles are never reused during a heap session. The language value
        // has no generation bits, so monotonic indices are the only sound way
        // to prevent a swept stale handle from resolving to a later object.
        self.objs.push(Some(object));
        self.layout_tags.push(layout);
        Ok(Value::from_heap(index))
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

    /// Transactionally mutate one object while preserving deterministic
    /// estimated-byte accounting. The old object is restored if the closure
    /// fails, accounting overflows, or the configured live-heap limit would be
    /// exceeded. Positive growth contributes to aggregate allocated bytes;
    /// shrinkage reduces only current live bytes.
    pub fn mutate<T>(
        &mut self,
        value: Value,
        mutation: impl FnOnce(&mut HeapObj) -> Result<T>,
    ) -> Result<T> {
        let index = value
            .as_heap()
            .ok_or_else(|| Error::msg("expected heap value"))? as usize;
        let old = self
            .objs
            .get(index)
            .and_then(Option::as_ref)
            .map(clone_object_for_transaction)
            .ok_or_else(|| Error::msg("bad heap index"))?;
        let old_bytes = estimated_object_bytes(&old);
        let result = {
            let object = self
                .objs
                .get_mut(index)
                .and_then(Option::as_mut)
                .ok_or_else(|| Error::msg("bad heap index"))?;
            mutation(object)
        };
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                self.objs[index] = Some(old);
                return Err(error);
            }
        };
        let new_object = self
            .objs
            .get(index)
            .and_then(Option::as_ref)
            .ok_or_else(|| Error::msg("bad heap index"))?;
        if !same_object_layout(&old, new_object) {
            self.objs[index] = Some(old);
            return Err(Error::msg("heap mutation changed object layout"));
        }
        let new_bytes = estimated_object_bytes(new_object);
        if new_bytes > old_bytes {
            let growth = new_bytes - old_bytes;
            let Some(next_live) = self.stats.live_heap_bytes.checked_add(growth) else {
                self.objs[index] = Some(old);
                return Err(Error::resource(
                    ResourceLimitKind::HeapBytes,
                    "heap mutation byte accounting overflow",
                ));
            };
            if next_live > self.config.max_heap_bytes {
                self.objs[index] = Some(old);
                return Err(Error::resource(
                    ResourceLimitKind::HeapBytes,
                    "heap mutation exceeds live heap byte limit",
                ));
            }
            self.stats.live_heap_bytes = next_live;
            self.stats.peak_live_heap_bytes = self.stats.peak_live_heap_bytes.max(next_live);
            self.stats.allocated_bytes = self
                .stats
                .allocated_bytes
                .saturating_add(u64::try_from(growth).unwrap_or(u64::MAX));
        } else {
            self.stats.live_heap_bytes = self
                .stats
                .live_heap_bytes
                .saturating_sub(old_bytes - new_bytes);
        }
        Ok(result)
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

    /// Build an owned transitive reachable snapshot without consuming the
    /// session heap. Stable indices are preserved and every unreachable slot
    /// remains absent from the returned storage.
    pub fn snapshot(&self, root: Value) -> Result<OwnedValue> {
        let marked = self.marked(&[root]);
        let snapshot = self
            .objs
            .iter()
            .zip(marked)
            .map(|(object, retain)| retain.then(|| object.clone()).flatten())
            .collect();
        OwnedValue::from_vm_snapshot(root, snapshot)
    }

    fn marked(&self, roots: &[Value]) -> Vec<bool> {
        let mut marked = vec![false; self.objs.len()];
        let mut pending = roots.to_vec();
        while let Some(value) = pending.pop() {
            let Some(index) = value.as_heap().map(|index| index as usize) else {
                continue;
            };
            if index >= self.objs.len() || marked[index] {
                continue;
            }
            let Some(object) = self.objs[index].as_ref() else {
                continue;
            };
            marked[index] = true;
            object.trace(&mut |child| pending.push(child));
        }
        marked
    }
}

fn stable_index_available(slot_count: usize) -> bool {
    u32::try_from(slot_count).is_ok()
}

fn gc_limit_error(limit: GcLimit) -> Error {
    match limit {
        GcLimit::Allocations => Error::resource(
            ResourceLimitKind::Allocations,
            "heap allocation or stable handle limit exceeded",
        ),
        GcLimit::HeapBytes => Error::resource(
            ResourceLimitKind::HeapBytes,
            "heap live byte limit exceeded",
        ),
    }
}

fn same_object_layout(old: &HeapObj, new: &HeapObj) -> bool {
    match (old, new) {
        (HeapObj::Int(_), HeapObj::Int(_))
        | (HeapObj::Float(_), HeapObj::Float(_))
        | (HeapObj::Str(_), HeapObj::Str(_))
        | (HeapObj::Symbol(_), HeapObj::Symbol(_))
        | (HeapObj::Pair { .. }, HeapObj::Pair { .. })
        | (HeapObj::Closure { .. }, HeapObj::Closure { .. })
        | (HeapObj::Builtin(_), HeapObj::Builtin(_))
        | (HeapObj::Buf(_), HeapObj::Buf(_))
        | (HeapObj::ResultOk(_), HeapObj::ResultOk(_))
        | (HeapObj::ResultErr(_), HeapObj::ResultErr(_))
        | (HeapObj::OptionSome(_), HeapObj::OptionSome(_)) => true,
        (
            HeapObj::Product {
                product: old_product,
                ..
            },
            HeapObj::Product {
                product: new_product,
                ..
            },
        ) => old_product == new_product,
        _ => false,
    }
}

fn clone_object_for_transaction(object: &HeapObj) -> HeapObj {
    fn clone_string(text: &str, capacity: usize) -> String {
        let mut clone = String::with_capacity(capacity);
        clone.push_str(text);
        clone
    }
    fn clone_values(values: &[Value], capacity: usize) -> Vec<Value> {
        let mut clone = Vec::with_capacity(capacity);
        clone.extend_from_slice(values);
        clone
    }
    match object {
        HeapObj::Int(value) => HeapObj::Int(*value),
        HeapObj::Float(value) => HeapObj::Float(*value),
        HeapObj::Str(text) => HeapObj::Str(clone_string(text, text.capacity())),
        HeapObj::Symbol(text) => HeapObj::Symbol(clone_string(text, text.capacity())),
        HeapObj::Pair { car, cdr } => HeapObj::Pair {
            car: *car,
            cdr: *cdr,
        },
        HeapObj::Closure { proto, captures } => HeapObj::Closure {
            proto: *proto,
            captures: clone_values(captures, captures.capacity()),
        },
        HeapObj::Builtin(value) => HeapObj::Builtin(*value),
        HeapObj::Buf(bytes) => {
            let mut clone = Vec::with_capacity(bytes.capacity());
            clone.extend_from_slice(bytes);
            HeapObj::Buf(clone)
        }
        HeapObj::ResultOk(value) => HeapObj::ResultOk(*value),
        HeapObj::ResultErr(value) => HeapObj::ResultErr(*value),
        HeapObj::OptionSome(value) => HeapObj::OptionSome(*value),
        HeapObj::Product { product, fields } => HeapObj::Product {
            product: *product,
            fields: clone_values(fields, fields.capacity()),
        },
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
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn collection_preserves_nested_graph_and_reports_exact_counters() {
        let mut heap = GcHeap::new(GcConfig {
            collect_before_every_allocation: true,
            ..GcConfig::default()
        });
        let payload = heap
            .alloc(HeapObj::Str("nested".into()))
            .expect("payload allocation");
        let list = heap
            .alloc(HeapObj::Pair {
                car: payload,
                cdr: Value::EMPTY_LIST,
            })
            .expect("list allocation");
        let result = heap
            .alloc(HeapObj::ResultOk(list))
            .expect("result allocation");
        let option = heap
            .alloc(HeapObj::OptionSome(result))
            .expect("option allocation");
        let product = heap
            .alloc(HeapObj::Product {
                product: crate::ProductId::new(0),
                fields: vec![option],
            })
            .expect("product allocation");
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
    fn stable_index_boundary_model_rejects_before_duplicate_u32_handles() {
        let last_valid_slot_count = u32::MAX as usize;
        assert!(stable_index_available(last_valid_slot_count));
        if let Some(exhausted_slot_count) = last_valid_slot_count.checked_add(1) {
            assert!(!stable_index_available(exhausted_slot_count));
        }
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
        assert!(matches!(
            heap.alloc(HeapObj::Str("compatibility".into())),
            Err(ref error)
                if error.class()
                    == crate::ErrorClass::Resource(ResourceLimitKind::Allocations)
        ));
        assert_eq!(heap.total_allocations(), 1);
    }

    #[test]
    fn swept_same_layout_handle_is_never_reused() {
        let mut heap = GcHeap::default();
        let stale = heap
            .try_alloc_with_layout(HeapObj::Buf(vec![1]), 77)
            .expect("first typed allocation");
        heap.collect(&[]);
        let current = heap
            .try_alloc_with_layout(HeapObj::Buf(vec![2]), 77)
            .expect("second typed allocation");
        assert_ne!(stale.as_heap(), current.as_heap());
        assert!(heap.get(stale).is_err());
        assert!(matches!(heap.get(current), Ok(HeapObj::Buf(bytes)) if bytes == &[2]));
    }

    #[test]
    fn mutation_growth_is_bounded_and_failure_rolls_back_every_object_kind() {
        for object in [
            HeapObj::Buf(vec![1]),
            HeapObj::Str("x".into()),
            HeapObj::Product {
                product: crate::ProductId::new(0),
                fields: vec![Value::UNIT],
            },
        ] {
            let mut heap = GcHeap::default();
            let value = heap.alloc(object.clone()).expect("test object allocation");
            heap.set_config(GcConfig {
                max_heap_bytes: heap.heap_bytes(),
                ..heap.config()
            });
            let result = heap.mutate(value, |current| {
                match current {
                    HeapObj::Buf(bytes) => bytes.extend_from_slice(&[2; 128]),
                    HeapObj::Str(text) => text.push_str(&"y".repeat(128)),
                    HeapObj::Product { fields, .. } => fields.extend([Value::UNIT; 128]),
                    _ => return Err(Error::msg("unexpected test object")),
                }
                Ok(())
            });
            assert!(matches!(
                result,
                Err(ref error)
                    if error.class() == crate::ErrorClass::Resource(ResourceLimitKind::HeapBytes)
            ));
            assert_eq!(heap.get(value).ok(), Some(&object));
        }

        let mut heap = GcHeap::default();
        let value = heap
            .alloc(HeapObj::Buf(vec![1]))
            .expect("growth buffer allocation");
        let before_growth = heap.stats();
        heap.mutate(value, |object| {
            let HeapObj::Buf(bytes) = object else {
                return Err(Error::msg("unexpected test object"));
            };
            bytes.extend_from_slice(&[2; 128]);
            Ok(())
        })
        .expect("bounded mutation growth");
        let after_growth = heap.stats();
        assert!(after_growth.live_heap_bytes > before_growth.live_heap_bytes);
        assert!(after_growth.allocated_bytes > before_growth.allocated_bytes);
        assert!(after_growth.peak_live_heap_bytes >= after_growth.live_heap_bytes);

        let mut heap = GcHeap::default();
        let value = heap
            .alloc(HeapObj::Buf(vec![1, 2]))
            .expect("rollback buffer allocation");
        let before = heap.stats();
        let result: Result<()> = heap.mutate(value, |object| {
            let HeapObj::Buf(bytes) = object else {
                return Err(Error::msg("unexpected test object"));
            };
            bytes.push(3);
            Err(Error::msg("reject mutation"))
        });
        assert!(result.is_err());
        assert_eq!(heap.get(value).ok(), Some(&HeapObj::Buf(vec![1, 2])));
        assert_eq!(heap.stats(), before);

        let result = heap.mutate(value, |object| {
            *object = HeapObj::Str("wrong layout".into());
            Ok(())
        });
        assert!(
            matches!(result, Err(ref error) if error.as_str() == "heap mutation changed object layout")
        );
        assert_eq!(heap.get(value).ok(), Some(&HeapObj::Buf(vec![1, 2])));
        assert_eq!(heap.stats(), before);
    }

    #[test]
    fn snapshot_clones_only_transitively_reachable_objects() {
        let mut heap = GcHeap::default();
        let child = heap
            .alloc(HeapObj::Str("child".into()))
            .expect("snapshot child allocation");
        let root = heap
            .alloc(HeapObj::OptionSome(child))
            .expect("snapshot root allocation");
        let _unreachable = heap
            .alloc(HeapObj::Str("unreachable".into()))
            .expect("snapshot unreachable allocation");
        let snapshot = heap.snapshot(root).expect("reachable snapshot");
        assert_eq!(snapshot.snapshot_object_count(), 2);
        assert!(heap.get(root).is_ok());
        assert_eq!(heap.total_allocations(), 3);
    }
}
