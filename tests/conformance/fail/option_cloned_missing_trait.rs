// expect: reject E_TRAIT
// witness: {"rustAccept":false,"rustCode":"E0277"}
struct Value { value: i64 }
pub fn probe() -> i64 { let value = Value { value: 42 }; Some(&value).cloned().unwrap().value }
