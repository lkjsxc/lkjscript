// expect: reject E_TRAIT
// witness: {"rustCode":"E0277"}
trait Mark { }
impl<T> Mark for [T] { }
fn take<T: Mark>(_value: T) { }
pub fn probe() -> i64 { let values = vec![1i64]; take(values); 42 }
