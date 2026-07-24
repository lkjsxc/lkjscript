//! Precise mark-sweep arena for heap objects.

use lkjscript_core::{Error, HeapObj, OwnedValue, Result, Value};

#[derive(Debug, Default)]
pub struct Arena {
    objs: Vec<Option<HeapObj>>,
    free: Vec<u32>,
    allocs_since_gc: u32,
    total_allocations: u64,
    heap_bytes: usize,
}

impl Arena {
    pub fn alloc(&mut self, obj: HeapObj) -> Value {
        self.allocs_since_gc = self.allocs_since_gc.saturating_add(1);
        self.total_allocations = self.total_allocations.saturating_add(1);
        self.heap_bytes = self.heap_bytes.saturating_add(estimated_object_bytes(&obj));
        if let Some(idx) = self.free.pop() {
            self.objs[idx as usize] = Some(obj);
            return Value::from_heap(idx);
        }
        let idx = self.objs.len() as u32;
        self.objs.push(Some(obj));
        Value::from_heap(idx)
    }

    pub fn get(&self, v: Value) -> Result<&HeapObj> {
        let idx = v
            .as_heap()
            .ok_or_else(|| Error::msg("expected heap value"))? as usize;
        self.objs
            .get(idx)
            .and_then(|o| o.as_ref())
            .ok_or_else(|| Error::msg("bad heap index"))
    }

    pub fn get_mut(&mut self, v: Value) -> Result<&mut HeapObj> {
        let idx = v
            .as_heap()
            .ok_or_else(|| Error::msg("expected heap value"))? as usize;
        self.objs
            .get_mut(idx)
            .and_then(|o| o.as_mut())
            .ok_or_else(|| Error::msg("bad heap index"))
    }

    /// Mark-sweep from roots. Safe to call any time.
    pub fn collect(&mut self, roots: &[Value]) {
        let n = self.objs.len();
        let mut marked = vec![false; n];
        let mut stack: Vec<Value> = roots.to_vec();
        while let Some(v) = stack.pop() {
            if let Some(i) = v.as_heap() {
                let i = i as usize;
                if i < n && !marked[i] {
                    marked[i] = true;
                    if let Some(Some(obj)) = self.objs.get(i) {
                        obj.trace(&mut |c| stack.push(c));
                    }
                }
            }
        }
        self.free.clear();
        for (i, slot) in self.objs.iter_mut().enumerate() {
            if let Some(object) = slot.as_ref().filter(|_| !marked[i]) {
                self.heap_bytes = self
                    .heap_bytes
                    .saturating_sub(estimated_object_bytes(object));
                *slot = None;
                self.free.push(i as u32);
            }
        }
        self.allocs_since_gc = 0;
    }

    pub fn needs_collect(&self) -> bool {
        self.allocs_since_gc >= 1024
    }

    pub const fn heap_bytes(&self) -> usize {
        self.heap_bytes
    }

    pub const fn total_allocations(&self) -> u64 {
        self.total_allocations
    }

    pub fn into_owned(mut self, root: Value) -> Result<OwnedValue> {
        self.collect(&[root]);
        OwnedValue::from_vm_snapshot(root, self.objs)
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
    fn collects_unreachable() {
        let mut a = Arena::default();
        let keep = a.alloc(HeapObj::Str("keep".into()));
        let _drop = a.alloc(HeapObj::Str("drop".into()));
        a.collect(&[keep]);
        assert!(a.get(keep).is_ok());
        assert_eq!(a.free.len(), 1);
    }

    #[test]
    fn option_some_traces_its_payload() {
        let mut arena = Arena::default();
        let payload = arena.alloc(HeapObj::Str("kept by some".into()));
        let some = arena.alloc(HeapObj::OptionSome(payload));
        arena.collect(&[some]);
        assert!(arena.get(some).is_ok());
        assert!(arena.get(payload).is_ok());
    }

    #[test]
    fn product_traces_every_nested_payload() {
        let mut arena = Arena::default();
        let payload = arena.alloc(HeapObj::Str("nested".into()));
        let inner_product = arena.alloc(HeapObj::Product {
            product: lkjscript_core::ProductId::new(1),
            fields: vec![payload],
        });
        let list = arena.alloc(HeapObj::Pair {
            car: inner_product,
            cdr: Value::EMPTY_LIST,
        });
        let result = arena.alloc(HeapObj::ResultOk(list));
        let option = arena.alloc(HeapObj::OptionSome(result));
        let outer_product = arena.alloc(HeapObj::Product {
            product: lkjscript_core::ProductId::new(0),
            fields: vec![option],
        });
        arena.collect(&[outer_product]);
        for value in [outer_product, option, result, list, inner_product, payload] {
            assert!(arena.get(value).is_ok());
        }
    }

    #[test]
    fn collection_resets_pressure_for_large_arenas() {
        let mut arena = Arena::default();
        let roots: Vec<_> = (0..4097)
            .map(|n| arena.alloc(HeapObj::Float(f64::from(n))))
            .collect();
        assert!(arena.needs_collect());
        arena.collect(&roots);
        assert!(!arena.needs_collect());
    }
}
