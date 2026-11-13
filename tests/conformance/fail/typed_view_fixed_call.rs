fn test<'a, 'b: 'a, F: FnMut(&'b i64)>(input: &'b i64, mut f: F) -> i64 {
    let pointer = input;
    let short: &&'a i64 = &pointer;
    f(*short);
    42
}
pub fn probe() -> i64 {
    let value = 42i64;
    test(&value, |_: &i64| {})
}
