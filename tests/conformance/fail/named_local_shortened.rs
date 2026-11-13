fn test<'a, 'b: 'a>(input: &'b i64) {
    let short: &'a i64 = input;
    let longer: &'b i64 = short;
    drop(longer);
}
pub fn probe() -> i64 {
    42
}
