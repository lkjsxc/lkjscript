//! An ordered set uses the map's single ordering and storage implementation.
use crate::BTreeMap;
use crate::iter::{IntoIterator, Iterator};

#[derive(Clone)]
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
    pub fn first(&self) -> crate::Option<&T> {
        match self.entries.first_key_value() {
            crate::Option::Some((key, _)) => crate::Option::Some(key),
            crate::Option::None => crate::Option::None,
        }
    }
    pub fn pop_first(&mut self) -> crate::Option<T> {
        match self.entries.pop_first() {
            crate::Option::Some((key, _)) => crate::Option::Some(key),
            crate::Option::None => crate::Option::None,
        }
    }
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

impl<T: Ord> crate::prelude::Default for BTreeSet<T> {
    fn default() -> BTreeSet<T> {
        BTreeSet::new()
    }
}
impl<T: Ord> PartialEq for BTreeSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}
impl<T: Ord> Eq for BTreeSet<T> {}

pub struct IntoIter<T> {
    inner: crate::ordered_map::IntoIter<T, ()>,
}
impl<T> crate::iter::Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> crate::Option<T> {
        match self.inner.next() {
            crate::Option::Some((key, _)) => crate::Option::Some(key),
            crate::Option::None => crate::Option::None,
        }
    }
}
impl<T> crate::iter::IntoIterator for IntoIter<T> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}
impl<T> crate::iter::IntoIterator for BTreeSet<T> {
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter {
            inner: self.entries.into_iter(),
        }
    }
}
