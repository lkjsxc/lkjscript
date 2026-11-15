fn recurse(n: usize) -> i64 {
    let frame = [n as i64; 1024];
    if n == 0 { return frame[0]; }
    recurse(n - 1) + frame[n % 1024]
}
pub fn probe() -> i64 {
    recurse(100000)
}
