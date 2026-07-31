use super::{
    allocation::{estimated_object_bytes, legacy_payload_is_valid},
    GcHeap,
};
use crate::{Error, HeapObj, ResourceLimitKind, Result, Value};

impl GcHeap {
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
            .as_legacy_traced()
            .ok_or_else(|| Error::msg("expected heap value"))? as usize;
        let source = self
            .objs
            .get(index)
            .and_then(Option::as_ref)
            .ok_or_else(|| Error::msg("bad heap index"))?;
        let old = clone_object_for_transaction(source);
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
        if !legacy_payload_is_valid(new_object) {
            self.objs[index] = Some(old);
            return Err(Error::msg(
                "legacy traced object cannot contain deterministic owners or capabilities",
            ));
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
}

fn same_object_layout(old: &HeapObj, new: &HeapObj) -> bool {
    match (old, new) {
        (HeapObj::Pair { .. }, HeapObj::Pair { .. }) => true,
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
        (
            HeapObj::Enum {
                layout: old_layout,
                physical_tag: old_tag,
                ..
            },
            HeapObj::Enum {
                layout: new_layout,
                physical_tag: new_tag,
                ..
            },
        ) => old_layout == new_layout && old_tag == new_tag,
        _ => false,
    }
}

fn clone_object_for_transaction(object: &HeapObj) -> HeapObj {
    fn clone_values(values: &[Value], capacity: usize) -> Vec<Value> {
        let mut clone = Vec::with_capacity(capacity);
        clone.extend_from_slice(values);
        clone
    }
    match object {
        HeapObj::Pair { car, cdr } => HeapObj::Pair {
            car: *car,
            cdr: *cdr,
        },
        HeapObj::Product { product, fields } => HeapObj::Product {
            product: *product,
            fields: clone_values(fields, fields.capacity()),
        },
        HeapObj::Enum {
            layout,
            physical_tag,
            active_payload,
        } => HeapObj::Enum {
            layout: *layout,
            physical_tag: *physical_tag,
            active_payload: clone_values(active_payload, active_payload.capacity()),
        },
    }
}
