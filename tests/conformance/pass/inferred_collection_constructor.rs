use std::collections::{BTreeMap, BTreeSet};
struct Holder<T> { value: T }
impl<T> Holder<T> { fn new(value: T) -> Self { Self { value } } }
pub fn probe() -> i64 {
    let mut values = BTreeMap::new();
    values.insert(5i32, 31i64);
    let mut keys = BTreeSet::new();
    keys.insert(5i32);
    let held = Holder::new(11i64);
    if keys.contains(&5i32) { *values.get(&5i32).unwrap() + held.value } else { 0 }
}
