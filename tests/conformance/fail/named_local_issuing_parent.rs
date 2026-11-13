fn bad<'a, 'b>(slot: &'b mut &'a mut i64) {
    let view: &'a mut i64 = &mut **slot;
    *view = 42;
}
pub fn probe() -> i64 {
    42
}
