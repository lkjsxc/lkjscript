//! An ordered set uses the map's single ordering and storage implementation.
use crate::BTreeMap;

pub struct BTreeSet<T> {
    entries: BTreeMap<T, ()>,
}

impl<T: Ord> BTreeSet<T> {
    pub fn new() -> BTreeSet<T> {
        BTreeSet {
            entries: BTreeMap::new(),
        }
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

/// Set iteration shares the map's key cursor and ordering.
pub type Iter<'a, T> = crate::ordered_map::Keys<'a, T, ()>;
impl<T> BTreeSet<T> {
    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        self.entries.keys()
    }
}
impl<'a, T> crate::iter::IntoIterator for &'a BTreeSet<T> {
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}
