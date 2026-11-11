trait Score { fn score(&self) -> i64; }
impl Score for i64 { fn score(&self) -> i64 { *self } }
struct Wrapped<T> { item: T }
impl<T: Score> Score for Wrapped<T> {
    fn score(&self) -> i64 { self.item.score() }
}
pub fn probe() -> i64 { Wrapped { item: 42i64 }.score() }
