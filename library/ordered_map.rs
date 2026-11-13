//! An AVL tree owns entries in an indexed arena. No references live in nodes.
//! Searches, insertion and removal take logarithmic tree height; vacant slots
//! are reused. Shared cursors follow parent links in key order.
use crate::Option;

pub struct BTreeMap<K, V> {
    nodes: Vec<Node<K, V>>,
    root: usize,
    vacant: usize,
    count: usize,
}

struct Pair<K, V> {
    key: K,
    value: V,
}

struct Node<K, V> {
    entry: Option<Pair<K, V>>,
    left: usize,
    right: usize,
    parent: usize,
    height: usize,
}

impl<K, V> BTreeMap<K, V> {
    pub fn new() -> BTreeMap<K, V> {
        BTreeMap {
            nodes: Vec::new(),
            root: 0,
            vacant: 0,
            count: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn first_node(&self, mut index: usize) -> usize {
        if index != 0 {
            while self.nodes[index].left != 0 {
                index = self.nodes[index].left;
            }
        }
        index
    }
    fn successor(&self, mut index: usize) -> usize {
        if self.nodes[index].right != 0 {
            return self.first_node(self.nodes[index].right);
        }
        let mut parent = self.nodes[index].parent;
        while parent != 0 && self.nodes[parent].right == index {
            index = parent;
            parent = self.nodes[index].parent;
        }
        parent
    }
    fn height(&self, index: usize) -> usize {
        if index == 0 {
            0
        } else {
            self.nodes[index].height
        }
    }
    fn update_height(&mut self, index: usize) {
        let left = self.height(self.nodes[index].left);
        let right = self.height(self.nodes[index].right);
        self.nodes[index].height = 1usize + if left > right { left } else { right };
    }
    fn replace_parent(&mut self, old: usize, new: usize) {
        let parent = self.nodes[old].parent;
        if parent == 0 {
            self.root = new;
        } else if self.nodes[parent].left == old {
            self.nodes[parent].left = new;
        } else {
            self.nodes[parent].right = new;
        }
        if new != 0 {
            self.nodes[new].parent = parent;
        }
    }
    fn rotate_left(&mut self, index: usize) -> usize {
        let top = self.nodes[index].right;
        let transferred = self.nodes[top].left;
        self.replace_parent(index, top);
        self.nodes[index].right = transferred;
        if transferred != 0 {
            self.nodes[transferred].parent = index;
        }
        self.nodes[top].left = index;
        self.nodes[index].parent = top;
        self.update_height(index);
        self.update_height(top);
        top
    }
    fn rotate_right(&mut self, index: usize) -> usize {
        let top = self.nodes[index].left;
        let transferred = self.nodes[top].right;
        self.replace_parent(index, top);
        self.nodes[index].left = transferred;
        if transferred != 0 {
            self.nodes[transferred].parent = index;
        }
        self.nodes[top].right = index;
        self.nodes[index].parent = top;
        self.update_height(index);
        self.update_height(top);
        top
    }
    fn balance(&mut self, mut index: usize) {
        while index != 0 {
            self.update_height(index);
            let left = self.nodes[index].left;
            let right = self.nodes[index].right;
            let left_height = self.height(left);
            let right_height = self.height(right);
            if left_height > right_height + 1usize {
                if self.height(self.nodes[left].left) < self.height(self.nodes[left].right) {
                    self.rotate_left(left);
                }
                index = self.rotate_right(index);
            } else if right_height > left_height + 1usize {
                if self.height(self.nodes[right].right) < self.height(self.nodes[right].left) {
                    self.rotate_right(right);
                }
                index = self.rotate_left(index);
            }
            index = self.nodes[index].parent;
        }
    }
    fn allocate(&mut self, key: K, value: V, parent: usize) -> usize {
        let node = Node {
            entry: Option::Some(Pair { key, value }),
            left: 0,
            right: 0,
            parent,
            height: 1,
        };
        if self.vacant != 0 {
            let index = self.vacant;
            self.vacant = self.nodes[index].right;
            self.nodes[index] = node;
            index
        } else {
            if self.nodes.len() == 0 {
                // Zero denotes no node; the first physical slot is always empty.
                self.nodes.push(Node {
                    entry: Option::None,
                    left: 0,
                    right: 0,
                    parent: 0,
                    height: 0,
                });
            }
            let index = self.nodes.len();
            self.nodes.push(node);
            index
        }
    }
    fn remove_node(&mut self, mut index: usize) -> Pair<K, V> {
        let removed = self.nodes[index].entry.take().unwrap();
        if self.nodes[index].left != 0 && self.nodes[index].right != 0 {
            let next = self.first_node(self.nodes[index].right);
            self.nodes[index].entry = self.nodes[next].entry.take();
            index = next;
        }
        let parent = self.nodes[index].parent;
        let child = if self.nodes[index].left != 0 {
            self.nodes[index].left
        } else {
            self.nodes[index].right
        };
        self.replace_parent(index, child);
        self.nodes[index].left = 0;
        self.nodes[index].right = self.vacant;
        self.nodes[index].parent = 0;
        self.nodes[index].height = 0;
        self.vacant = index;
        self.count = self.count - 1usize;
        self.balance(parent);
        removed
    }
    pub fn clear(&mut self) {
        let mut index = self.first_node(self.root);
        while index != 0 {
            let next = self.successor(index);
            // Rust drops ordered-map keys and values in key order.
            self.nodes[index].entry.take();
            index = next;
        }
        self.nodes = Vec::new();
        self.root = 0;
        self.vacant = 0;
        self.count = 0;
    }
    pub fn iter<'a>(&'a self) -> Iter<'a, K, V> {
        Iter {
            map: self,
            next: self.first_node(self.root),
        }
    }
    pub fn keys<'a>(&'a self) -> Keys<'a, K, V> {
        Keys { inner: self.iter() }
    }
    pub fn values<'a>(&'a self) -> Values<'a, K, V> {
        Values { inner: self.iter() }
    }
}

impl<K: Ord, V> BTreeMap<K, V> {
    fn compare_key(&self, index: usize, key: &K) -> i32 {
        match &self.nodes[index].entry {
            Option::Some(entry) => {
                if *key < entry.key {
                    -1
                } else if *key > entry.key {
                    1
                } else {
                    0
                }
            }
            Option::None => unreachable!(),
        }
    }
    fn find_node(&self, key: &K) -> usize {
        let mut index = self.root;
        while index != 0 {
            let ordering = self.compare_key(index, key);
            if ordering < 0 {
                index = self.nodes[index].left;
            } else if ordering > 0 {
                index = self.nodes[index].right;
            } else {
                return index;
            }
        }
        0
    }
    pub fn contains_key(&self, key: &K) -> bool {
        self.find_node(key) != 0
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        let index = self.find_node(key);
        if index == 0 {
            Option::None
        } else {
            match &self.nodes[index].entry {
                Option::Some(entry) => Option::Some(&entry.value),
                Option::None => unreachable!(),
            }
        }
    }
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let index = self.find_node(key);
        if index == 0 {
            Option::None
        } else {
            match &mut self.nodes[index].entry {
                Option::Some(entry) => Option::Some(&mut entry.value),
                Option::None => unreachable!(),
            }
        }
    }
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let mut parent = 0usize;
        let mut index = self.root;
        let mut left = false;
        while index != 0 {
            parent = index;
            let ordering = self.compare_key(index, &key);
            if ordering < 0 {
                left = true;
                index = self.nodes[index].left;
            } else if ordering > 0 {
                left = false;
                index = self.nodes[index].right;
            } else {
                match &mut self.nodes[index].entry {
                    Option::Some(entry) => {
                        return Option::Some(std::mem::replace(&mut entry.value, value));
                    }
                    Option::None => unreachable!(),
                }
            }
        }
        let index = self.allocate(key, value, parent);
        if parent == 0 {
            self.root = index;
        } else if left {
            self.nodes[parent].left = index;
        } else {
            self.nodes[parent].right = index;
        }
        self.count = self.count + 1usize;
        self.balance(parent);
        Option::None
    }
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let index = self.find_node(key);
        if index == 0 {
            Option::None
        } else {
            Option::Some(self.remove_node(index).value)
        }
    }
}

impl<K, V> Drop for BTreeMap<K, V> {
    fn drop(&mut self) {
        self.clear();
    }
}

pub struct Iter<'a, K, V> {
    map: &'a BTreeMap<K, V>,
    next: usize,
}
impl<'a, K, V> crate::iter::Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.next == 0 {
            return Option::None;
        }
        let index = self.next;
        self.next = self.map.successor(index);
        match &self.map.nodes[index].entry {
            Option::Some(entry) => Option::Some((&entry.key, &entry.value)),
            Option::None => unreachable!(),
        }
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
