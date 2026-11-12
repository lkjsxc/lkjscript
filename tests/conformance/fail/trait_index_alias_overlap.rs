trait Value { fn value(&self) -> i64; }
struct Owner { value: i64 }
type Alias = Owner;
impl Value for Alias { fn value(&self) -> i64 { self.value } }
impl Value for Owner { fn value(&self) -> i64 { self.value } }
pub fn probe() -> i64 { 42 }
