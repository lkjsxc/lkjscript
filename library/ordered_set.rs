//! An ordered set uses the map's single ordering and storage implementation.
use crate::BTreeMap;

pub struct BTreeSet<T> {
    entries: BTreeMap<T, ()>,
}

impl<T: Ord> BTreeSet<T> {
    pub fn new() -> BTreeSet<T> {
        BTreeSet { entries: BTreeMap::new() }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn contains(&self, value: &T) -> bool {
        self.entries.contains_key(value)
    }
    pub fn insert(&mut self, value: T) -> bool {
        self.entries.insert(value, ()).is_none()
    }
    pub fn remove(&mut self, value: &T) -> bool {
        self.entries.remove(value).is_some()
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
