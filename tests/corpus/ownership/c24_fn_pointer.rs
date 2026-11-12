fn inc(x: i64) -> i64 { x + 1 }
fn apply(f: fn(i64) -> i64, x: i64) -> i64 { f(x) }
pub fn probe() -> i64 { apply(inc, 41) }
