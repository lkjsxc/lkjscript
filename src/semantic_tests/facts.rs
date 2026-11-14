use crate::facts::Facts;
fn indices(facts: &Facts, start: usize) -> Vec<usize> {
    let mut values = Vec::new();
    let mut iterator = facts.indices_from(start);
    while let Some(value) = iterator.next() {
        values.push(value);
    }
    values
}
pub fn run() -> usize {
    let mut facts = Facts::new(8193);
    for index in vec![0usize, 63, 64, 4095, 4096, 8192] {
        facts.set(index, true);
    }
    let original = facts.clone();
    facts.set(4096, false);
    facts.resize(1000001, false);
    facts.set(1000000, true);
    assert!(original.at(4096) && !facts.at(4096));
    assert!(indices(&facts, 64) == vec![1000000usize, 8192, 4095, 64]);
    facts.truncate(8193);
    facts.set(4096, true);
    assert!(facts == original);
    facts.truncate(64);
    assert!(indices(&facts, 0) == vec![63usize, 0]);
    facts.resize(8193, false);
    assert!(!facts.at(64) && !facts.at(8192) && original.at(8192));

    let mut facts = Facts::default();
    let mut model = Vec::new();
    let mut seed = 0x8413u64;
    for step in 0usize..4000usize {
        seed = seed.wrapping_mul(6364136223846793005u64).wrapping_add(1u64);
        let size = (seed as usize % 520usize) + 1usize;
        if step % 5usize == 0usize {
            let value = step % 10usize == 0usize;
            facts.resize(size, value);
            while model.len() > size {
                model.pop();
            }
            while model.len() < size {
                model.push(value);
            }
        } else {
            let index = seed as usize % facts.len().max(1usize);
            if index < facts.len() {
                let value = step % 3usize == 0usize;
                facts.set(index, value);
                model[index] = value;
            }
        }
        let start = size / 3usize;
        let mut expected = Vec::new();
        let mut at = model.len();
        while at > start {
            at -= 1;
            if model[at] {
                expected.push(at);
            }
        }
        assert!(indices(&facts, start) == expected);
        assert!(facts.any() == model.iter().any(|value| *value));
        for index in 0usize..model.len() {
            assert!(facts.at(index) == model[index]);
        }
    }
    let mut left = Facts::new(257);
    let mut right = Facts::new(130);
    for i in 0usize..257usize {
        left.set(i, i % 3usize == 0usize || i % 11usize == 0usize);
    }
    for i in 0usize..130usize {
        right.set(i, i % 7usize == 0usize || i % 11usize == 0usize);
    }
    let combined = vec![
        left.intersection(&right),
        left.union(&right),
        left.symmetric_difference(&right),
    ];
    for kind in 0usize..3usize {
        for i in 0usize..257usize {
            let a = i % 3usize == 0usize || i % 11usize == 0usize;
            let b = i < 130usize && (i % 7usize == 0usize || i % 11usize == 0usize);
            let expected = match kind {
                0 => a && b,
                1 => a || b,
                _ => a != b,
            };
            assert!(combined[kind].at(i) == expected);
        }
    }
    assert!(left == left.union(&left) && left == left.intersection(&left));
    assert!(!left.symmetric_difference(&left).any());
    3
}
