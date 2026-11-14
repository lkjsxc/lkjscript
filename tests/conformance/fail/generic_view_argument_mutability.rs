// expect: reject E_TYPE
// witness: {"rustCode":"E0308"}
fn visit<T>(values: &mut [T]) { let _ = values.len(); }
pub fn probe() -> i64 { let values = vec![1i64]; visit(&values); 42 }
