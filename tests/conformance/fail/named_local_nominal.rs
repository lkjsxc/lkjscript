struct Holder<'x> {
    pointer: &'x mut i64,
}
fn test<'a>(value: &'a mut i64) {
    let pointer: Holder<'a> = Holder {
        pointer: &mut *value,
    };
    drop(pointer);
    *value = 42;
}
pub fn probe() -> i64 {
    42
}
