trait Score { fn score(&self) -> i64; }
struct Ticket { value: i64 }
impl Score for Ticket { fn score(&self) -> i64 { self.value } }
fn read<T: Score>(value: &T) -> i64 { value.score() }
pub fn probe() -> i64 { read(&Ticket { value: 42 }) }
