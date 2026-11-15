// expect: reject E_BORROW
// witness: {"rustCode":"E0502"}
pub fn probe() -> i64 {
    let mut values = vec![42i64];
    for value in &values {
        values.push(*value);
    }
    0
}
