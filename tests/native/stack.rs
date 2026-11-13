fn depth(n: i64) -> i64 {
    if n == 0 { 0 } else { depth(n - 1) + 1 }
}
pub fn probe() -> i64 { depth(10000) }
