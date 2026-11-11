//! An ordered map API over a sorted vector. Lookup is logarithmic; insertion is linear.
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
