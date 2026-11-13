fn first(n: i64) -> i64 {
    second(n + 1) + 1
}
fn second(n: i64) -> i64 {
    first(n + 1) + 1
}
pub fn probe() -> i64 {
    first(0)
}
