fn test<'a, 'b>(left: &'a mut i64, right: &'b mut i64) {
    let mut pointer = &mut *right;
    pointer = &mut *left;
    let holder: &&'a mut i64 = &pointer;
    drop(holder);
    drop(pointer);
}
pub fn probe() -> i64 {
    42
}
