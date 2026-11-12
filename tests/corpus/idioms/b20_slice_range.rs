fn sum(xs: &[i64]) -> i64 { let mut s: i64 = 0; let mut i: usize = 0; while i < xs.len() { s += xs[i]; i += 1; } s }
pub fn probe() -> i64 { let v = vec![1i64, 40, 2, 9]; sum(&v[1..3]) }
