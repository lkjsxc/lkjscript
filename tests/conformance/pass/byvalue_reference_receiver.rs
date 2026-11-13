trait Take {
    fn take(self) -> i64;
}
impl<'a> Take for &'a i64 {
    fn take(self) -> i64 {
        *self
    }
}
impl<'a> Take for &'a mut i64 {
    fn take(self) -> i64 {
        *self = *self + 1;
        *self
    }
}
fn apply<T: Take>(value: T) -> i64 {
    value.take()
}
pub fn probe() -> i64 {
    let value = 42i64;
    assert_eq!(apply(&value), 42);
    let mut value = 41i64;
    apply(&mut value)
}
