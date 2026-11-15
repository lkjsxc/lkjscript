fn consume(v: Vec<i64>) -> i64 { v.len() as i64 }
 pub fn probe() -> i64 { let mut v: Vec<i64> = Vec::new(); let mut n: i64 = 0; let mut i: i64 = 0; while i < 42 { v.push(i); n = n + consume(v); v = Vec::new(); i = i + 1; } n }
