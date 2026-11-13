fn test<'a, 'b: 'a>() {
    let short: Option<&'a i64> = None;
    let longer: Option<&'b i64> = short;
    drop(longer);
}
pub fn probe() -> i64 {
    42
}
