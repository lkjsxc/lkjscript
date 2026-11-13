fn bad<'a>(value: &'a mut i64) {
    let Some(pointer): Option<&'a mut i64> = Some(&mut *value) else {
        return;
    };
    drop(pointer);
    *value = 42;
}
pub fn probe() -> i64 {
    42
}
