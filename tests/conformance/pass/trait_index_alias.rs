trait Value { fn value(&self) -> i64; }
trait Forward { fn forward(&self) -> i64; }
struct Left { value: i64 }
struct Right { value: i64 }
type Alias = Left;
impl Value for Alias { fn value(&self) -> i64 { self.value } }
impl Value for Right { fn value(&self) -> i64 { self.value } }
impl<T: Value> Forward for T { fn forward(&self) -> i64 { self.value() } }
fn get<T: Forward>(value: &T) -> i64 { value.forward() }
pub fn probe() -> i64 { get(&Left { value: 20 }) + get(&Right { value: 22 }) }
