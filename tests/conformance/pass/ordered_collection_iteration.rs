use std::collections::{BTreeMap, BTreeSet};
fn retained<'a>(map: &'a BTreeMap<i64, i64>) -> (&'a i64, &'a i64) {
    let mut iterator = map.iter();
    let first = iterator.next().unwrap();
    let second = iterator.next().unwrap();
    assert!(iterator.next().is_none());
    assert!(iterator.next().is_none());
    drop(iterator);
    assert_eq!(*first.0, 1);
    assert_eq!(*second.0, 2);
    (first.1, second.1)
}
pub fn probe() -> i64 {
    let mut map: BTreeMap<i64, i64> = BTreeMap::new();
    map.insert(2i64, 22i64);
    map.insert(1i64, 20i64);
    let (first, second) = retained(&map);
    assert_eq!(*first + *second, 42);
    let mut total = 0i64;
    for (key, value) in &map {
        total = total + *key + *value;
    }
    assert_eq!(total, 45);
    let mut keys = 0i64;
    for key in map.keys() {
        keys = keys + *key;
    }
    assert_eq!(keys, 3);
    let mut values = 0i64;
    for value in map.values() {
        values = values + *value;
    }
    assert_eq!(values, 42);
    let mut set: BTreeSet<i64> = BTreeSet::new();
    set.insert(22i64);
    set.insert(20i64);
    set.insert(22i64);
    let mut iterator = set.iter();
    let first = iterator.next().unwrap();
    let second = iterator.next().unwrap();
    assert!(iterator.next().is_none());
    drop(iterator);
    assert_eq!(*first + *second, 42);
    let mut result = 0i64;
    for value in &set {
        result = result + *value;
    }
    result
}
