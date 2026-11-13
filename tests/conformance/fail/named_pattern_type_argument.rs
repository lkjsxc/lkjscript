struct Wrap<T> {
    value: T,
}
fn test<'a, 'b: 'a>(input: &'b i64) -> &'b i64 {
    let Wrap::<&'a i64> { value } = Wrap { value: input };
    value
}
pub fn probe() -> i64 {
    42
}
