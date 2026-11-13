fn test<'a, 'b: 'a>(input: &'b i64) -> &'b i64 {
    let pointer = input;
    let short: &&'a i64 = &pointer;
    *short
}
pub fn probe() -> i64 {
    let value = 42i64;
    *test(&value)
}
