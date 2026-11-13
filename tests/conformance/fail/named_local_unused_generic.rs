fn bad<'a, T>(value: &'a mut T) {
    let pointer: &'a mut T = &mut *value;
    drop(pointer);
    let other = &mut *value;
    drop(other);
}
pub fn probe() -> i64 {
    42
}
