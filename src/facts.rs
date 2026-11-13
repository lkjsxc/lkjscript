//! Compiler facts packed into words. Bits at or beyond len are zero and words
//! holds exactly ceil(len / 64) entries, so derived equality is semantic equality.
//! Snapshots copy the words rather than retaining shared nodes.

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct Facts {
    words: Vec<u64>,
    len: usize,
}

fn word_count(len: usize) -> usize {
    len / 64 + if len % 64 == 0 { 0 } else { 1 }
}

// The caller supplies a nonzero word. A fixed binary search uses only the
// ordinary integer operations available to the compiler itself.
fn highest_bit(mut word: u64) -> usize {
    let mut bit = 0usize;
    if word >> 32u64 != 0 {
        word >>= 32u64;
        bit += 32;
    }
    if word >> 16u64 != 0 {
        word >>= 16u64;
        bit += 16;
    }
    if word >> 8u64 != 0 {
        word >>= 8u64;
        bit += 8;
    }
    if word >> 4u64 != 0 {
        word >>= 4u64;
        bit += 4;
    }
    if word >> 2u64 != 0 {
        word >>= 2u64;
        bit += 2;
    }
    if word >> 1u64 != 0 {
        bit += 1;
    }
    bit
}

impl Facts {
    pub(crate) fn new(len: usize) -> Self {
        Self {
            words: vec![0u64; word_count(len)],
            len,
        }
    }

    pub(crate) fn filled(len: usize, value: bool) -> Self {
        let mut result = Self::default();
        result.resize(len, value);
        result
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn get(&self, index: usize) -> Option<bool> {
        if index >= self.len {
            return None;
        }
        Some(self.words[index / 64] & (1u64 << ((index % 64) as u64)) != 0)
    }

    pub(crate) fn at(&self, index: usize) -> bool {
        self.get(index).expect("local index out of bounds")
    }

    pub(crate) fn set(&mut self, index: usize, value: bool) {
        assert!(index < self.len);
        let bit = 1u64 << ((index % 64) as u64);
        if value {
            self.words[index / 64] |= bit;
        } else {
            self.words[index / 64] &= !bit;
        }
    }

    pub(crate) fn push(&mut self, value: bool) {
        let index = self.len;
        let len = self.len.checked_add(1).expect("local index overflow");
        self.resize(len, false);
        self.set(index, value);
    }

    pub(crate) fn resize(&mut self, len: usize, value: bool) {
        if len < self.len {
            self.truncate(len);
            return;
        }
        let old_len = self.len;
        let count = word_count(len);
        while self.words.len() < count {
            self.words.push(0u64);
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
        let count = word_count(len);
        while self.words.len() > count {
            self.words.pop();
        }
        if len % 64 != 0 {
            let last = self.words.len() - 1;
            self.words[last] &= !0u64 >> ((64 - len % 64) as u64);
        }
        self.len = len;
    }

    fn combined(&self, other: &Self, kind: u8) -> Self {
        let mut result = self.clone();
        result.resize(self.len.max(other.len), false);
        for index in 0..result.words.len() {
            let left = result.words[index];
            let right = if index < other.words.len() {
                other.words[index]
            } else {
                0u64
            };
            result.words[index] = match kind {
                0 => left & right,
                1 => left | right,
                _ => left ^ right,
            };
        }
        result
    }

    pub(crate) fn intersection(&self, other: &Self) -> Self {
        self.combined(other, 0)
    }

    pub(crate) fn union(&self, other: &Self) -> Self {
        self.combined(other, 1)
    }

    pub(crate) fn symmetric_difference(&self, other: &Self) -> Self {
        self.combined(other, 2)
    }

    pub(crate) fn any(&self) -> bool {
        self.words.iter().any(|word| *word != 0)
    }

    /// Live indices at or above start, in reverse declaration order. Empty words
    /// are skipped without visiting each local slot.
    pub(crate) fn indices_from(&self, start: usize) -> Indices<'_> {
        Indices {
            words: &self.words,
            end: self.len,
            start,
        }
    }
}

pub(crate) struct Indices<'a> {
    words: &'a [u64],
    end: usize,
    start: usize,
}
impl Iterator for Indices<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        while self.end > self.start {
            let index = self.end - 1;
            let base = index - index % 64;
            let word = self.words[index / 64] & (!0u64 >> ((63 - index % 64) as u64));
            if word == 0 {
                self.end = base;
                continue;
            }
            let found = base + highest_bit(word);
            if found < self.start {
                self.end = self.start;
                return None;
            }
            self.end = found;
            return Some(found);
        }
        None
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
        assert!(original.at(4096));
        assert!(!facts.at(4096));
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
        assert!(!facts.at(64));
        assert!(!facts.at(8192));
        assert!(original.at(8192));
    }

    #[test]
    fn sparse_facts_match_boolean_model_at_word_boundaries() {
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
                assert_eq!(facts.at(index), *value);
            }
        }
    }
    #[test]
    fn combinations_match_independent_bits_across_word_boundaries() {
        let mut left = Facts::new(257);
        let mut right = Facts::new(130);
        for i in 0..257 {
            left.set(i, i % 3 == 0 || i % 11 == 0);
        }
        for i in 0..130 {
            right.set(i, i % 7 == 0 || i % 11 == 0);
        }
        for (kind, actual) in [
            left.intersection(&right),
            left.union(&right),
            left.symmetric_difference(&right),
        ]
        .iter()
        .enumerate()
        {
            for i in 0..257 {
                let a = i % 3 == 0 || i % 11 == 0;
                let b = i < 130 && (i % 7 == 0 || i % 11 == 0);
                assert_eq!(
                    actual.at(i),
                    match kind {
                        0 => a && b,
                        1 => a || b,
                        _ => a != b,
                    }
                );
            }
        }
        assert!(left == left.union(&left));
        assert!(left == left.intersection(&left));
        assert!(!left.symmetric_difference(&left).any());
    }
}
