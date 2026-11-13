//! An ordered map API over a sorted vector. Lookup is logarithmic; insertion is linear.
use crate::Option;

pub struct BTreeMap<K, V> {
    entries: Vec<Entry<K, V>>,
}

struct Entry<K, V> {
    key: K,
    value: V,
}

impl<K: Ord, V> BTreeMap<K, V> {
    pub fn new() -> BTreeMap<K, V> {
        BTreeMap {
            entries: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }

    pub fn clear(&mut self) {
        self.entries = Vec::new();
    }

    fn lower_bound(&self, key: &K) -> usize {
        let mut low: usize = 0;
        let mut high: usize = self.entries.len();
        while low < high {
            let middle = low + (high - low) / 2;
            if self.entries[middle].key < *key {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        low
    }

    pub fn contains_key(&self, key: &K) -> bool {
        let index = self.lower_bound(key);
        if index < self.entries.len() {
            self.entries[index].key == *key
        } else {
            false
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let index = self.lower_bound(key);
        if index < self.entries.len() {
            if self.entries[index].key == *key {
                return Option::Some(&self.entries[index].value);
            }
        }
        Option::None
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let index = self.lower_bound(key);
        if index < self.entries.len() {
            if self.entries[index].key == *key {
                return Option::Some(&mut self.entries[index].value);
            }
        }
        Option::None
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let index = self.lower_bound(&key);
        if index < self.entries.len() {
            if self.entries[index].key == key {
                return Option::Some(std::mem::replace(&mut self.entries[index].value, value));
            }
        }
        let mut pending = Entry { key, value };
        let mut position = index;
        while position < self.entries.len() {
            pending = std::mem::replace(&mut self.entries[position], pending);
            position = position + 1;
        }
        self.entries.push(pending);
        Option::None
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let index = self.lower_bound(key);
        if index == self.entries.len() {
            return Option::None;
        }
        if self.entries[index].key != *key {
            return Option::None;
        }
        let mut tail: Vec<Entry<K, V>> = Vec::new();
        while self.entries.len() > index + 1 {
            tail.push(self.entries.pop().unwrap());
        }
        let removed = self.entries.pop().unwrap();
        while tail.len() > 0 {
            self.entries.push(tail.pop().unwrap());
        }
        Option::Some(removed.value)
    }
}

/// A shared cursor over entries in key order. Item borrows outlive the cursor.
pub struct Iter<'a, K, V> {
    entries: &'a [Entry<K, V>],
    index: usize,
}

impl<K, V> BTreeMap<K, V> {
    pub fn iter<'a>(&'a self) -> Iter<'a, K, V> {
        Iter {
            entries: self.entries.as_slice(),
            index: 0,
        }
    }
    pub fn keys<'a>(&'a self) -> Keys<'a, K, V> {
        Keys { inner: self.iter() }
    }
    pub fn values<'a>(&'a self) -> Values<'a, K, V> {
        Values { inner: self.iter() }
    }
}

impl<'a, K, V> crate::iter::Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.index == self.entries.len() {
            return Option::None;
        }
        let index = self.index;
        self.index = index + 1;
        let entry = &self.entries[index];
        Option::Some((&entry.key, &entry.value))
    }
}
impl<'a, K, V> crate::iter::IntoIterator for Iter<'a, K, V> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}
impl<'a, K, V> crate::iter::IntoIterator for &'a BTreeMap<K, V> {
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

pub struct Keys<'a, K, V> {
    inner: Iter<'a, K, V>,
}
impl<'a, K, V> crate::iter::Iterator for Keys<'a, K, V> {
    type Item = &'a K;
    fn next(&mut self) -> Option<&'a K> {
        match self.inner.next() {
            Option::Some((key, _)) => Option::Some(key),
            Option::None => Option::None,
        }
    }
}
impl<'a, K, V> crate::iter::IntoIterator for Keys<'a, K, V> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

pub struct Values<'a, K, V> {
    inner: Iter<'a, K, V>,
}
impl<'a, K, V> crate::iter::Iterator for Values<'a, K, V> {
    type Item = &'a V;
    fn next(&mut self) -> Option<&'a V> {
        match self.inner.next() {
            Option::Some((_, value)) => Option::Some(value),
            Option::None => Option::None,
        }
    }
}
impl<'a, K, V> crate::iter::IntoIterator for Values<'a, K, V> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}
