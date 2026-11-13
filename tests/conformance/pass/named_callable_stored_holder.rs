fn invoke<'a, F: FnMut(&&'a mut i64)>(value: &'a mut i64, mut f: F) -> i64 {
    let pointer = &mut *value;
    f(&pointer);
    *pointer = *pointer + 1;
    *pointer
}
pub fn probe() -> i64 {
    let mut value = 41i64;
    invoke(&mut value, |pointer: &&mut i64| {
        assert_eq!(**pointer, 41);
    })
}
