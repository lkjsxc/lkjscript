//! Bump arena for heap objects.

use lkjscript2026_core::{Error, HeapObj, Result, Value};

#[derive(Debug, Default)]
pub struct Arena {
    objs: Vec<HeapObj>,
}

impl Arena {
    pub fn alloc(&mut self, obj: HeapObj) -> Value {
        let idx = self.objs.len() as u32;
        self.objs.push(obj);
        Value::from_heap(idx)
    }

    pub fn get(&self, v: Value) -> Result<&HeapObj> {
        let idx = v
            .as_heap()
            .ok_or_else(|| Error::msg("expected heap value"))? as usize;
        self.objs
            .get(idx)
            .ok_or_else(|| Error::msg("bad heap index"))
    }

    pub fn get_mut(&mut self, v: Value) -> Result<&mut HeapObj> {
        let idx = v
            .as_heap()
            .ok_or_else(|| Error::msg("expected heap value"))? as usize;
        self.objs
            .get_mut(idx)
            .ok_or_else(|| Error::msg("bad heap index"))
    }
}
