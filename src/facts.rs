//! Persistent compiler facts. Sharing is an implementation choice, not a language
//! capacity limit: the tree grows with the highest declared local.
use std::ops::Index;
use std::rc::Rc;

type Link = Option<Rc<Node>>;

#[derive(Clone)]
enum Node {
    Word(u64),
    Branch { left: Link, right: Link },
}

/// A boolean vector whose snapshots share untouched words and subtrees.
#[derive(Clone, Default)]
pub(crate) struct Facts {
    root: Link,
    depth: usize,
    len: usize,
}

fn depth_for(len: usize) -> usize {
    if len <= 64 {
        0
    } else {
        (usize::BITS - ((len - 1) / 64).leading_zeros()) as usize
    }
}

fn equal(a: &Link, b: &Link) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            if Rc::ptr_eq(a, b) {
                return true;
            }
            match (&**a, &**b) {
                (Node::Word(a), Node::Word(b)) => a == b,
                (
                    Node::Branch {
                        left: al,
                        right: ar,
                    },
                    Node::Branch {
                        left: bl,
                        right: br,
                    },
                ) => equal(al, bl) && equal(ar, br),
                _ => false,
            }
        }
        _ => false,
    }
}

fn set_word(link: &mut Link, depth: usize, index: usize, bit: u64, value: bool) {
    if link.is_none() {
        if !value {
            return;
        }
        *link = Some(Rc::new(if depth == 0 {
            Node::Word(0)
        } else {
            Node::Branch {
                left: None,
                right: None,
            }
        }));
    }
    let node = Rc::make_mut(link.as_mut().unwrap());
    let empty = match node {
        Node::Word(word) => {
            if value {
                *word |= bit;
            } else {
                *word &= !bit;
            }
            *word == 0
        }
        Node::Branch { left, right } => {
            let next = if index & (1usize << (depth - 1)) == 0 {
                left
            } else {
                right
            };
            set_word(next, depth - 1, index, bit, value);
            match node {
                Node::Branch { left, right } => left.is_none() && right.is_none(),
                _ => unreachable!(),
            }
        }
    };
    if empty {
        *link = None;
    }
}

/// Remove a suffix. Entire subtrees outside the prefix disappear in one step.
fn retain_prefix(link: &mut Link, depth: usize, word: usize, bits: usize) {
    let Some(root) = link else { return };
    let node = Rc::make_mut(root);
    let empty = match node {
        Node::Word(value) => {
            *value &= if bits == 0 { 0 } else { (1u64 << bits) - 1 };
            *value == 0
        }
        Node::Branch { left, right } => {
            let half = 1usize << (depth - 1);
            if word < half {
                *right = None;
                retain_prefix(left, depth - 1, word, bits);
            } else {
                retain_prefix(right, depth - 1, word - half, bits);
            }
            left.is_none() && right.is_none()
        }
    };
    if empty {
        *link = None;
    }
}

impl Facts {
    pub(crate) fn new(len: usize) -> Self {
        Self {
            root: None,
            depth: depth_for(len),
            len,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn get(&self, index: usize) -> Option<bool> {
        if index >= self.len {
            return None;
        }
        let mut link = &self.root;
        let mut depth = self.depth;
        while let Some(node) = link {
            match &**node {
                Node::Word(word) => return Some(word & (1u64 << (index % 64)) != 0),
                Node::Branch { left, right } => {
                    depth -= 1;
                    link = if (index / 64) & (1usize << depth) == 0 {
                        left
                    } else {
                        right
                    };
                }
            }
        }
        Some(false)
    }

    pub(crate) fn set(&mut self, index: usize, value: bool) {
        assert!(index < self.len);
        if self[index] != value {
            set_word(
                &mut self.root,
                self.depth,
                index / 64,
                1u64 << (index % 64),
                value,
            );
        }
    }

    pub(crate) fn push(&mut self, value: bool) {
        let index = self.len;
        self.resize(
            self.len.checked_add(1).expect("local index overflow"),
            false,
        );
        self.set(index, value);
    }

    pub(crate) fn resize(&mut self, len: usize, value: bool) {
        if len < self.len {
            self.truncate(len);
            return;
        }
        let old_len = self.len;
        let depth = depth_for(len);
        while self.depth < depth {
            if self.root.is_some() {
                self.root = Some(Rc::new(Node::Branch {
                    left: self.root.take(),
                    right: None,
                }));
            }
            self.depth += 1;
        }
        self.len = len;
        if value {
            for index in old_len..len {
                self.set(index, true);
            }
        }
    }

    pub(crate) fn truncate(&mut self, len: usize) {
        if len >= self.len {
            return;
        }
        if len == 0 {
            self.root = None;
            self.depth = 0;
            self.len = 0;
            return;
        }
        retain_prefix(&mut self.root, self.depth, len / 64, len % 64);
        let depth = depth_for(len);
        while self.depth > depth {
            self.root = match &self.root {
                Some(root) => match &**root {
                    Node::Branch { left, .. } => left.clone(),
                    Node::Word(_) => unreachable!(),
                },
                None => None,
            };
            self.depth -= 1;
        }
        self.len = len;
    }

    pub(crate) fn any(&self) -> bool {
        self.root.is_some()
    }

    /// Live indices at or above start, in reverse declaration order. This skips
    /// entire empty subtrees rather than scanning historical local slots.
    pub(crate) fn indices_from(&self, start: usize) -> Indices<'_> {
        let mut pending = Vec::new();
        if start < self.len {
            if let Some(root) = &self.root {
                pending.push((&**root, 0, self.depth));
            }
        }
        Indices {
            pending,
            word: 0,
            base: 0,
            start,
        }
    }
}

impl PartialEq for Facts {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && equal(&self.root, &other.root)
    }
}
impl Eq for Facts {}

impl Index<usize> for Facts {
    type Output = bool;
    fn index(&self, index: usize) -> &bool {
        if self.get(index).expect("local index out of bounds") {
            &true
        } else {
            &false
        }
    }
}

impl FromIterator<bool> for Facts {
    fn from_iter<I: IntoIterator<Item = bool>>(values: I) -> Self {
        let mut result = Self::default();
        for value in values {
            result.push(value);
        }
        result
    }
}

pub(crate) struct Indices<'a> {
    pending: Vec<(&'a Node, usize, usize)>,
    word: u64,
    base: usize,
    start: usize,
}
impl Iterator for Indices<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        loop {
            if self.word != 0 {
                let bit = 63 - self.word.leading_zeros() as usize;
                self.word &= !(1u64 << bit);
                let index = self.base * 64 + bit;
                if index < self.start {
                    self.pending.clear();
                    self.word = 0;
                    return None;
                }
                return Some(index);
            }
            let (node, base, depth) = self.pending.pop()?;
            if base + (1usize << depth) <= self.start / 64 {
                continue;
            }
            match node {
                Node::Word(word) => {
                    self.word = *word;
                    self.base = base;
                }
                Node::Branch { left, right } => {
                    if let Some(left) = left {
                        self.pending.push((&**left, base, depth - 1));
                    }
                    if let Some(right) = right {
                        self.pending
                            .push((&**right, base + (1usize << (depth - 1)), depth - 1));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Facts;

    #[test]
    fn snapshots_preserve_sparse_facts_across_growth_and_restoration() {
        let mut facts = Facts::new(8_193);
        for index in [0, 63, 64, 4095, 4096, 8192] {
            facts.set(index, true);
        }
        let original = facts.clone();
        facts.set(4096, false);
        facts.resize(1_000_001, false);
        facts.set(1_000_000, true);
        assert!(original[4096]);
        assert!(!facts[4096]);
        assert_eq!(
            facts.indices_from(64).collect::<Vec<_>>(),
            vec![1_000_000, 8192, 4095, 64]
        );
        facts.truncate(8193);
        facts.set(4096, true);
        assert!(facts == original);
        facts.truncate(64);
        assert_eq!(facts.indices_from(0).collect::<Vec<_>>(), vec![63, 0]);
        facts.resize(8193, false);
        assert!(!facts[64]);
        assert!(!facts[8192]);
        assert!(original[8192]);
    }

    #[test]
    fn sparse_facts_match_boolean_model_at_word_and_tree_boundaries() {
        let mut facts = Facts::default();
        let mut model = Vec::new();
        let mut seed = 0x8413_u64;
        for step in 0..4_000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let size = (seed as usize % 520) + 1;
            if step % 5 == 0 {
                let value = step % 10 == 0;
                facts.resize(size, value);
                model.resize(size, value);
            } else {
                let index = seed as usize % facts.len().max(1);
                if index < facts.len() {
                    let value = step % 3 == 0;
                    facts.set(index, value);
                    model[index] = value;
                }
            }
            let start = size / 3;
            assert_eq!(
                facts.indices_from(start).collect::<Vec<_>>(),
                (start..model.len())
                    .rev()
                    .filter(|index| model[*index])
                    .collect::<Vec<_>>()
            );
            assert_eq!(facts.any(), model.iter().any(|value| *value));
            for (index, value) in model.iter().enumerate() {
                assert_eq!(facts[index], *value);
            }
        }
    }
}
