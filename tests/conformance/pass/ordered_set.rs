use std::collections::{BTreeMap, BTreeSet};

pub fn probe() -> i64 {
    let mut values: BTreeSet<i64> = BTreeSet::new();
    assert!(values.is_empty());
    assert!(values.insert(20));
    assert!(values.insert(42));
    assert!(!values.insert(20));
    assert!(values.len() == 2);
    assert!(values.contains(&42));
    assert!(values.remove(&20));
    assert!(!values.remove(&20));
    assert!(values.len() == 1);
    values.clear();
    assert!(values.is_empty());
    assert!(values.insert(42));
    let mut map: BTreeMap<i64, i64> = BTreeMap::new();
    map.insert(1, 99);
    map.clear();
    assert!(map.is_empty());
    if values.contains(&42) { 42 } else { 0 }
}
