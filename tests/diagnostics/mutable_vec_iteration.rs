// expect: reject E_TRAIT
// pending: Name the missing IntoIterator implementation for &mut Vec<T> and suggest an index loop borrowing one element at a time.
pub fn probe() -> i64 {
    let mut values = vec![1i64];
    for value in &mut values { *value += 1i64; }
    values[0]
}
