fn bad<T>(unused: T) -> i64 {
    match 0i64 {
        _ => 1,
        1 => true,
    }
}
pub fn probe() -> i64 {
    0
}
