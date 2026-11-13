fn test<'a, 'b>(left: &'a i64, right: &'b i64) {
    let mut pointer: &'a i64 = left;
    let slot = &mut pointer;
    *slot = right;
    drop(slot);
    drop(pointer);
}
pub fn probe() -> i64 {
    42
}
