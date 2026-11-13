trait Take {
    fn take(self) -> i64;
}
impl<'a> Take for &'a mut i64 {
    fn take(self) -> i64 {
        *self
    }
}
fn apply<T: Take>(value: T) -> i64 {
    value.take()
}
pub fn probe() -> i64 {
    let mut value = 42i64;
    let view = &mut value;
    let result = apply(view);
    result + *view
}
