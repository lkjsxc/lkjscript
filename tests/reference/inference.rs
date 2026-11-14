// expect: value 42
// Constraint examples formerly embedded in docs/language.md.
fn pair() -> (i64, i64) {
    (40, 2)
}
pub fn probe() -> i64 {
    let mut values = Vec::new();
    values.push(42i64);
    let mut index = 0;
    let mut result = 0i64;
    while index < values.len() {
        result += values[index];
        index += 1;
    }
    let (left, right) = pair();
    assert!(left + right == result);
    result
}
