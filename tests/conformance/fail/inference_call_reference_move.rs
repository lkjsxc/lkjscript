fn identity<T>(value: T) -> T { value }
fn invalid() {
    let mut value = 1i64;
    let reference = &mut value;
    let held = identity(reference);
    drop(held);
    *reference = 3;
}
