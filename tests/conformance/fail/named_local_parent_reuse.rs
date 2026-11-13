fn bad<'a, 'b: 'a>(slot: &'b mut &'a mut i64) {
    let view: &'a mut i64 = &mut **slot;
    drop(view);
    **slot = 42;
}
pub fn probe() -> i64 {
    42
}
