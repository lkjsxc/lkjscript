fn test<'a, 'b: 'a>(mut input: &'b i64) -> i64 {
    let short: &mut &'a i64 = &mut input;
    drop(short);
    42
}
pub fn probe() -> i64 {
    let value = 42i64;
    test(&value)
}
