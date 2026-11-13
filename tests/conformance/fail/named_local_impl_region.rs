struct Holder<'a> {
    value: &'a mut i64,
}
impl<'a> Holder<'a> {
    fn bad(self) {
        let pointer: &'a mut i64 = &mut *self.value;
        drop(pointer);
        *self.value = 42;
    }
}
pub fn probe() -> i64 {
    42
}
