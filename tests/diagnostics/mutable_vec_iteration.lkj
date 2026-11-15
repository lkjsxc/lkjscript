// expect: reject E_TRAIT
// rewrite: Name the missing IntoIterator for &mut Vec<T>; use one collection evaluation and an index loop with a fresh element borrow.
pub fn probe() -> i64 {
    let mut values = vec![1i64];
    for value in &mut values { *value += 1i64; }
    values[0]
}
