struct Holder<'x> {
    pointer: &'x i64,
}
fn test<'a, 'b>(left: &'a i64, right: &'b i64) {
    let mut holder: Holder<'a> = Holder { pointer: left };
    holder.pointer = right;
    drop(holder);
}
pub fn probe() -> i64 {
    42
}
