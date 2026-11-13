fn recurse(n: i64) -> i64 {
    recurse(n + 1) + 1
}
pub fn probe() -> i64 {
    recurse(0)
}
