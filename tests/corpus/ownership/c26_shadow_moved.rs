fn consume(v: Vec<i64>) -> i64 { v.len() as i64 }
 pub fn probe() -> i64 { let v: Vec<i64> = Vec::new(); let n = consume(v); let v: i64 = 42; v + n }
