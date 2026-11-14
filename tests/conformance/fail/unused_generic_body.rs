// expect: reject E_TYPE
// witness: {"rustAccept":false}
fn bad<T>(value: T) -> i64 { 1i64 + true }
pub fn probe() -> i64 { 0 }
