use std::collections::BTreeMap;
fn verify(map: &BTreeMap<usize, i64>, expected: &Vec<Option<i64>>) {
    let mut previous = 0usize;
    let mut count = 0usize;
    for (key, value) in map {
        if count > 0usize {
            assert!(previous < *key);
        }
        assert_eq!(*value, expected[*key].unwrap());
        previous = *key;
        count += 1usize;
    }
    let mut present = 0usize;
    let mut key = 0usize;
    while key < expected.len() {
        assert_eq!(map.get(&key).copied(), expected[key]);
        assert_eq!(map.contains_key(&key), expected[key].is_some());
        if expected[key].is_some() {
            present += 1usize;
        }
        key += 1usize;
    }
    assert_eq!(count, present);
    assert_eq!(map.len(), present);
    assert_eq!(map.is_empty(), present == 0usize);
}
pub fn probe() -> i64 {
    let mut map: BTreeMap<usize, i64> = BTreeMap::new();
    let mut expected: Vec<Option<i64>> = vec![None; 127usize];
    let mut index = 0usize;
    while index < 127usize {
        let key = 126usize - index;
        assert!(map.insert(key, key as i64).is_none());
        expected[key] = Some(key as i64);
        index += 1usize;
    }
    verify(&map, &expected);
    let mut state = 0x12345678u64;
    let mut step = 0usize;
    while step < 4096usize {
        state ^= state << 13u64;
        state ^= state >> 7u64;
        state ^= state << 17u64;
        let key = (state % 127u64) as usize;
        let action = (state >> 16u64) % 5u64;
        if action < 2u64 {
            let value = step as i64;
            let before = expected[key];
            expected[key] = Some(value);
            assert_eq!(map.insert(key, value), before);
        } else if action < 4u64 {
            let before = expected[key].take();
            assert_eq!(map.remove(&key), before);
        } else {
            if let Some(value) = map.get_mut(&key) {
                *value += 1i64;
                expected[key] = Some(expected[key].unwrap() + 1i64);
            } else {
                assert!(expected[key].is_none());
            }
        }
        if step % 97usize == 0usize {
            verify(&map, &expected);
        }
        step += 1usize;
    }
    verify(&map, &expected);
    let mut key = 0usize;
    while key < 127usize {
        assert_eq!(map.remove(&key), expected[key].take());
        key += 1usize;
    }
    assert!(map.is_empty());
    key = 0usize;
    while key < 127usize {
        map.insert(key, key as i64);
        key += 1usize;
    }
    map.clear();
    assert!(map.is_empty());
    map.insert(42usize, 42i64);
    *map.get(&42usize).unwrap()
}
