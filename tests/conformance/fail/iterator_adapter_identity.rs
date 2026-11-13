trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}
struct Wrong {}
impl Iterator for Wrong {
    type Item = i64;
    fn next(&mut self) -> Option<i64> {
        None
    }
}
pub fn probe() -> i64 {
    let _ = Wrong {}.enumerate();
    42
}
