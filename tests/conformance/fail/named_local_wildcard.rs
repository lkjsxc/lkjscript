fn test<'a>(value: &'a mut i64) {
    let _: &'a mut i64 = value;
    *value = 42;
}
pub fn probe() -> i64 {
    42
}
