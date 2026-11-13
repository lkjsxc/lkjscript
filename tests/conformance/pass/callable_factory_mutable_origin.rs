fn choose<T, F: FnOnce() -> T>(value: Option<T>, make: F) -> T {
    match value {
        Some(value) => value,
        None => make(),
    }
}
pub fn probe() -> i64 {
    let mut value = 41i64;
    let reference = &mut value;
    let missing: Option<&mut i64> = None;
    let result = choose(missing, move || {
        let moved = reference;
        moved
    });
    *result = *result + 1;
    *result
}
