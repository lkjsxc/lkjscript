// expect: reject E_TYPE
// witness: {"rustCode":"E0308"}
fn same<T>(_values: &[T], _value: T) { }
pub fn probe() -> i64 { let values = vec![1i64]; same(&values, true); 42 }
