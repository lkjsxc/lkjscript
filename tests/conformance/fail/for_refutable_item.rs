// expect: reject E_TYPE
// witness: {"rustCode":"E0005"}
pub fn probe() -> i64 {
    let values = vec![Some(42i64)];
    for Some(value) in &values {
        return *value;
    }
    0
}
