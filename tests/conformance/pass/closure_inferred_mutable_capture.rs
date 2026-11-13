fn tail(moved: bool) -> i64 {
    let mut value = 42i64;
    let reference = &mut value;
    if moved {
        let once = move || reference;
        *once()
    } else {
        let once = || reference;
        *once()
    }
}
fn explicit_return() -> i64 {
    let mut value = 42i64;
    let reference = &mut value;
    let once = move || { return reference; };
    *once()
}
fn branch(yes: bool) -> i64 {
    let mut left = 42i64;
    let mut right = 42i64;
    let first = &mut left;
    let second = &mut right;
    let once = move || { if yes { return first; } second };
    *once()
}
fn product() -> i64 {
    let mut value = 41i64;
    let reference = &mut value;
    let once = move || (reference, 1i64);
    let result = once();
    *result.0 = *result.0 + result.1;
    *result.0
}
fn choose<T, F: FnOnce() -> T>(value: Option<T>, make: F) -> T {
    match value { Some(value) => value, None => make() }
}
pub fn probe() -> i64 {
    assert_eq!(tail(true), 42);
    assert_eq!(tail(false), 42);
    assert_eq!(explicit_return(), 42);
    assert_eq!(branch(true), 42);
    assert_eq!(branch(false), 42);
    assert_eq!(product(), 42);
    let mut value = 42i64;
    let reference = &mut value;
    let missing: Option<&mut i64> = None;
    *choose(missing, move || reference)
}
