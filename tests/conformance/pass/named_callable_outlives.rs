fn invoke<'short, 'long: 'short, F: FnOnce(&'short i64) -> i64>(f: F, value: &'long i64) -> i64 {
    f(value)
}
pub fn probe() -> i64 {
    let value = 42i64;
    invoke(|input: &i64| *input, &value)
}
