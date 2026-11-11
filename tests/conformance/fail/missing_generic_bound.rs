trait Score {
    fn score(&self) -> i64;
}
fn ignore<T: Score>(value: T) -> i64 {
    42
}
pub fn probe() -> i64 {
    ignore::<i64>(0)
}
