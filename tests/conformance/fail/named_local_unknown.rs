// expect: reject E_TYPE
// witness: {"rustCode":"E0261"}
fn test(value: &i64) {
    let pointer: &'missing i64 = value;
    drop(pointer);
}
pub fn probe() -> i64 {
    42
}
