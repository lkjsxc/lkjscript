fn test<'a, 'b: 'a>(input: &'b i64) -> Option<&i64> {
    let pointer = input;
    let short: &&'a i64 = &pointer;
    Some(*short)
}
pub fn probe() -> i64 {
    let value = 42i64;
    *test(&value).unwrap()
}
