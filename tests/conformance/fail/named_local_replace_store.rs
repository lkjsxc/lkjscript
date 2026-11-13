fn test<'a, 'b>(left: &'a i64, right: &'b i64) {
    let mut pointer: &'a i64 = left;
    let old = std::mem::replace(&mut pointer, right);
    drop(old);
    drop(pointer);
}
pub fn probe() -> i64 {
    42
}
