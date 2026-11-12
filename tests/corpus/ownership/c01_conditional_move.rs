fn consume(v: Vec<i64>) -> i64 { v.len() as i64 }
 pub fn probe() -> i64 { let mut v: Vec<i64> = Vec::new(); v.push(1); let c: bool = v.len() > 0usize; let mut n: i64 = 41; if c { n = n + consume(v); } n }
