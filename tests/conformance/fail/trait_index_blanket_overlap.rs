trait Value { fn value(&self) -> i64; }
struct Owner { value: i64 }
impl<T> Value for T { fn value(&self) -> i64 { 0 } }
impl Value for Owner { fn value(&self) -> i64 { self.value } }
pub fn probe() -> i64 { 42 }
