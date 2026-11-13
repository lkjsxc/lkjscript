fn bad<'a>(value: &'a mut i64) {
    loop {
        let pointer: &'a mut i64 = &mut *value;
        drop(pointer);
        break;
    }
    *value = 42;
}
pub fn probe() -> i64 {
    42
}
