struct Held<T> { value: T }
fn invalid() {
    let mut value = 1i64;
    let reference = &mut value;
    let held = Held { value: reference };
    drop(held);
    *reference = 3;
}
