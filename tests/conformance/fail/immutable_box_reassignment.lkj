// expect: reject E_TYPE
// witness: {"rustAccept":false}
pub fn probe() -> i64 {
    let value = Box::new(1i64);
    value = Box::new(42i64);
    *value
}
