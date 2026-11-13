fn invoke<'a, F: FnMut(&'a i64) -> i64>(mut f: F, value: &'a i64) -> i64 {
    let mut result = 0i64;
    let mut index = 0usize;
    while index < 2 {
        result = result + f(value);
        index = index + 1;
    }
    result
}
pub fn probe() -> i64 {
    let value = 21i64;
    invoke(|input: &i64| *input, &value)
}
