use std::collections::BTreeMap;
pub fn probe() -> i64 {
    let mut map: BTreeMap<i64, i64> = BTreeMap::new();
    map.insert(1i64, 42i64);
    let mut iterator = map.iter();
    let value = iterator.next().unwrap();
    drop(iterator);
    map.insert(2i64, 1i64);
    *value.1
}
