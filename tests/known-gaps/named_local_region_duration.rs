// Rust rejects E0506; the current call-region verifier does not yet enforce the duration of named lifetime annotations on local bindings.
fn test<'a>(value: &'a mut i64) {
    let pointer: &'a mut i64 = &mut *value;
    drop(pointer);
    *value = 42;
}
pub fn probe() -> i64 {
    42
}
