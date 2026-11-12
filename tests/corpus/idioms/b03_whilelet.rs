pub fn probe() -> i64 { let mut v: Vec<i64> = Vec::new(); v.push(40); v.push(2); let mut s: i64 = 0; while let Some(x) = v.pop() { s = s + x; } s }
