fn test(value: &i64) {
    let pointer: &'missing i64 = value;
    drop(pointer);
}
pub fn probe() -> i64 {
    42
}
