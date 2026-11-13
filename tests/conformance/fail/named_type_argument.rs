fn identity<T>(value: T) -> T {
    value
}
fn test<'a, 'b: 'a>(input: &'b i64) -> i64 {
    let pointer = input;
    let short = identity::<&&'a i64>(&pointer);
    let long: &'b i64 = *short;
    *long
}
pub fn probe() -> i64 {
    let value = 42i64;
    test(&value)
}
