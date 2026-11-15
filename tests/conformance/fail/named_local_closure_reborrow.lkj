// expect: reject E_BORROW
// witness: {"rustCode":"E0506"}
fn test<'a>(value: &'a mut i64) {
    let act = || {
        let pointer: &'a mut i64 = &mut *value;
        drop(pointer);
    };
    drop(act);
    *value = 42;
}
pub fn probe() -> i64 {
    42
}
