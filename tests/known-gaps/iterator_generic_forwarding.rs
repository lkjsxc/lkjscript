use std::collections::btree_map::Iter;
fn next<I: Iterator>(iterator: &mut I) -> Option<I::Item> {
    iterator.next()
}
fn forward<'a, K, V>(iterator: &mut Iter<'a, K, V>) -> Option<(&'a K, &'a V)> {
    next(iterator)
}
pub fn probe() -> i64 {
    let mut values: std::collections::BTreeMap<i64, i64> = std::collections::BTreeMap::new();
    values.insert(1, 42);
    let mut iterator = values.iter();
    let value = forward(&mut iterator).unwrap();
    drop(iterator);
    *value.1
}
