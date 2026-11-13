fn choose<T, F: FnOnce() -> T>(value: Option<T>, make: F) -> T {
    match value {
        Some(value) => value,
        None => make(),
    }
}
pub fn probe() -> i64 {
    let result: &i64;
    {
        let short = 42i64;
        let missing: Option<&i64> = None;
        result = choose(missing, || &short);
    }
    *result
}
