fn test<'a, 'b: 'a>(input: &'b i64) -> i64 {
    let holder = (input, input);
    let short: &(&'a i64, &'b i64) = &holder;
    let long: &'b i64 = short.0;
    *long
}
pub fn probe() -> i64 {
    let input = 42i64;
    test(&input)
}
