struct Holder<'x> {
    value: &'x i64,
}
fn test<'a, 'b: 'a>(input: &'b i64) -> i64 {
    let holder = Holder { value: input };
    let short: &Holder<'a> = &holder;
    let long: &'b i64 = short.value;
    *long
}
pub fn probe() -> i64 {
    let input = 42i64;
    test(&input)
}
