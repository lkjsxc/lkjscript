fn test<'a>(left: &'a mut i64, right: &'a mut i64) {
    let mut pointer: &'a mut i64 = &mut *left;
    pointer = &mut *right;
    drop(pointer);
    *left = 42;
}
pub fn probe() -> i64 {
    42
}
