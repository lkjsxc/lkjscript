trait Score { fn score(&self) -> i64; }
struct Wrapped<T> { item: T }
impl<T> Score for Wrapped<T> { fn score(&self) -> i64 { 0 } }
impl Score for Wrapped<i64> { fn score(&self) -> i64 { self.item } }
pub fn probe() -> i64 { Wrapped { item: 42i64 }.score() }
