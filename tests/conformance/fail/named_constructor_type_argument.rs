struct Wrap<T> {
    value: T,
}
fn test<'a, 'b: 'a>(input: &'b i64) -> &'b i64 {
    let value = Wrap::<&'a i64> { value: input };
    value.value
}
pub fn probe() -> i64 {
    42
}
