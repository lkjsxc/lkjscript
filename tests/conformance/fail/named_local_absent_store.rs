fn test<'a, 'b>(right: &'b i64) {
    let mut pointer: Option<&'a i64> = None;
    pointer = Some(right);
    drop(pointer);
}
pub fn probe() -> i64 {
    42
}
