use super::{GcHeap, GcLimit};
use crate::{Error, HeapObj, ResourceLimitKind, Result, Value};

impl GcHeap {
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

    /// Check the complete generated enum publication before any scalar box or
    /// enum object is published. The caller executes synchronously after this
    /// check, so the exact aggregate reservation cannot be consumed elsewhere.
    #[doc(hidden)]
    pub fn preflight_enum_allocations(
        &self,
        scalar_boxes: usize,
        active_fields: usize,
    ) -> std::result::Result<(), GcLimit> {
        let allocation_count = scalar_boxes.checked_add(1).ok_or(GcLimit::Allocations)?;
        let allocation_count_u64 =
            u64::try_from(allocation_count).map_err(|_| GcLimit::Allocations)?;
        if self
            .stats
            .allocations
            .checked_add(allocation_count_u64)
            .is_none_or(|total| total > self.config.max_allocations)
        {
            return Err(GcLimit::Allocations);
        }
        let final_index = self
            .objs
            .len()
            .checked_add(allocation_count)
            .and_then(|length| length.checked_sub(1))
            .ok_or(GcLimit::Allocations)?;
        if !stable_index_available(final_index) {
            return Err(GcLimit::Allocations);
        }
        let base_bytes = std::mem::size_of::<HeapObj>()
            .checked_mul(allocation_count)
            .ok_or(GcLimit::HeapBytes)?;
        let payload_bytes = std::mem::size_of::<Value>()
            .checked_mul(active_fields)
            .ok_or(GcLimit::HeapBytes)?;
        let added = base_bytes
            .checked_add(payload_bytes)
            .ok_or(GcLimit::HeapBytes)?;
        if self
            .stats
            .live_heap_bytes
            .checked_add(added)
            .is_none_or(|total| total > self.config.max_heap_bytes)
        {
            return Err(GcLimit::HeapBytes);
        }
        Ok(())
    }

    /// Publish a boxed enum only after validating layout, physical tag, and
    /// exact active initialized payload against immutable metadata.
    pub fn alloc_validated_enum(
        &mut self,
        definition: &crate::EnumMetadata,
        layout: crate::RuntimeLayoutId,
        physical_tag: u16,
        active_payload: Vec<Value>,
    ) -> Result<Value> {
        if definition.layout != layout {
            return Err(Error::msg("enum allocation layout identity mismatch"));
        }
        let variant = definition
            .variants
            .iter()
            .find(|variant| variant.physical_tag == physical_tag)
            .ok_or_else(|| Error::msg("enum allocation physical tag is invalid"))?;
        if active_payload.len() != variant.fields.len() {
            return Err(Error::msg("enum allocation active payload is malformed"));
        }
        self.alloc(HeapObj::Enum {
            layout,
            physical_tag,
            active_payload,
        })
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
}

pub(super) fn stable_index_available(slot_count: usize) -> bool {
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

pub(super) fn estimated_object_bytes(object: &HeapObj) -> usize {
    let base = std::mem::size_of::<HeapObj>();
    let dynamic = match object {
        HeapObj::Str(text) | HeapObj::Symbol(text) => text.capacity(),
        HeapObj::Pair { .. } | HeapObj::Int(_) | HeapObj::Float(_) | HeapObj::Builtin(_) => 0,
        HeapObj::Closure { captures, .. } => captures
            .capacity()
            .saturating_mul(std::mem::size_of::<Value>()),
        HeapObj::Buf(bytes) => bytes.capacity(),
        HeapObj::Product { fields, .. } => fields
            .capacity()
            .saturating_mul(std::mem::size_of::<Value>()),
        HeapObj::Enum { active_payload, .. } => active_payload
            .capacity()
            .saturating_mul(std::mem::size_of::<Value>()),
    };
    base.saturating_add(dynamic)
}
