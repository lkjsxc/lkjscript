// expect: value 42
struct Value {
    number: i64,
}
impl Clone for Value {
    fn clone(&self) -> Self {
        Value {
            number: self.number,
        }
    }
}
impl Value {
    fn clone(&self) -> Self {
        Value { number: 99 }
    }
}
fn duplicate<T: Clone>(value: &T) -> T {
    value.clone()
}
pub fn probe() -> i64 {
    let value = Value { number: 42 };
    assert!(value.clone().number == 99);
    duplicate(&value).number
}
