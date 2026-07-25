use super::{allocation::estimated_object_bytes, GcHeap};
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
