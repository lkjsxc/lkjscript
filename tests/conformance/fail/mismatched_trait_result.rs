trait Score {
    fn score(&self) -> i64;
}
impl Score for bool {
    fn score(&self) -> bool {
        *self
    }
}
pub fn probe() -> i64 {
    42
}
