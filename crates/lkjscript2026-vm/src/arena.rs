//! Precise mark-sweep arena for heap objects.

use lkjscript2026_core::{Error, HeapObj, Result, Value};

#[derive(Debug, Default)]
pub struct Arena {
    objs: Vec<Option<HeapObj>>,
    free: Vec<u32>,
    allocs_since_gc: u32,
}

impl Arena {
    pub fn alloc(&mut self, obj: HeapObj) -> Value {
        self.allocs_since_gc += 1;
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
            if slot.is_some() && !marked[i] {
                *slot = None;
                self.free.push(i as u32);
            }
        }
        self.allocs_since_gc = 0;
    }

    pub fn needs_collect(&self) -> bool {
        self.allocs_since_gc >= 1024
    }

    pub fn maybe_collect(&mut self, roots: &[Value]) {
        if self.needs_collect() {
            self.collect(roots);
        }
    }
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
