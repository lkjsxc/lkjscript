fn test<'a>(left: &'a mut i64) {
    let mut value = 0i64;
    let mut pointer = &mut value;
    pointer = &mut *left;
    let holder: &&'a mut i64 = &pointer;
    drop(holder);
    drop(pointer);
}
pub fn probe() -> i64 {
    42
}
