fn invoke<'a, F: FnOnce(&'a i64)>(f: F, value: &'a mut i64) -> i64 {
    f(&*value);
    *value
}
pub fn probe() -> i64 {
    let mut value = 42i64;
    invoke(
        |input: &i64| {
            assert_eq!(*input, 42);
        },
        &mut value,
    )
}
