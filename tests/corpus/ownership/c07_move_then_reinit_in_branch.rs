fn consume(v: Vec<i64>) -> i64 { v.len() as i64 }
 pub fn probe() -> i64 { let mut v: Vec<i64> = Vec::new(); v.push(1); let mut n: i64 = 40; if n > 0 { n = n + consume(v); v = Vec::new(); } v.push(1); n + v.len() as i64 }
