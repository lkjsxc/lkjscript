// expect: reject E_TRAIT
// witness: {"rustAccept":false,"rustCode":"E0599"}
fn append<T>(target: &mut Vec<T>, source: &[T]) { target.extend_from_slice(source); }
pub fn probe() -> i64 { 42 }
