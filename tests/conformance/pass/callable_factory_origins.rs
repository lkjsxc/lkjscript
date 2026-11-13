fn choose<T, F: FnOnce() -> T>(value: Option<T>, make: F) -> T {
    match value {
        Some(value) => value,
        None => make(),
    }
}
pub fn probe() -> i64 {
    let left = 20i64;
    let right = 22i64;
    let mut calls = 0i64;
    let first = choose(Some(&left), || {
        calls = calls + 1;
        &right
    });
    let missing: Option<&i64> = None;
    let second = choose(missing, || {
        calls = calls + 1;
        &right
    });
    assert_eq!(calls, 1);
    *first + *second
}
