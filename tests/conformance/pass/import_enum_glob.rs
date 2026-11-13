mod values {
    pub enum Choice { Empty, Pair(i64, i64), Named { value: i64 } }
}
mod use_values {
    use super::values::{self, Choice};
    use Choice::*;
    pub fn score(value: values::Choice) -> i64 {
        match value { Empty => 1, Pair(a,b) => a+b, Named { value } => value }
    }
    pub fn run() -> i64 { score(Pair(20,21)) + score(Empty) + score(Named { value:0 }) }
}
pub fn probe() -> i64 { use_values::run() }
