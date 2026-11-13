use std::collections::BTreeSet;
pub fn probe() -> i64 {
    let mut values: BTreeSet<i64> = BTreeSet::new();
    values.insert(42);
    let mut iterator = values.iter();
    let value = iterator.next().unwrap();
    drop(iterator);
    drop(values);
    *value
}
