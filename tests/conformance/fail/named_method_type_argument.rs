struct Maker;
impl Maker {
    fn wrap<T>(&self, value: T) -> T {
        value
    }
}
fn test<'a, 'b: 'a>(input: &'b i64) -> &'b i64 {
    let maker = Maker;
    maker.wrap::<&'a i64>(input)
}
pub fn probe() -> i64 {
    42
}
