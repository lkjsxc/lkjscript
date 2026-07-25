use super::GcHeap;
use crate::{Error, HeapObj, OwnedValue, Result, Value};

impl GcHeap {
    pub fn get(&self, value: Value) -> Result<&HeapObj> {
        let index = value
            .as_heap()
            .ok_or_else(|| Error::msg("expected heap value"))? as usize;
        self.objs
            .get(index)
            .and_then(Option::as_ref)
            .ok_or_else(|| Error::msg("bad heap index"))
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
