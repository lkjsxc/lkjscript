trait Substitute {
    fn len(&self) -> bool;
}
impl<T> Substitute for Vec<T> {
    fn len(&self) -> bool {
        true
    }
}
pub fn probe() -> i64 {
    let values = vec![20i64, 22i64];
    let size = values.len();
    let known: usize = size;
    known as i64 + 40
}
