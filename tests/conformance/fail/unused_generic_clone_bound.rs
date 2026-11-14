// expect: reject E_TRAIT
// witness: {"rustAccept":false}
fn copied<T>(value: T) -> T { value.clone() }
pub fn probe() -> i64 { 0 }
