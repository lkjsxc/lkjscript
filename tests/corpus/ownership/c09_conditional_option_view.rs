pub fn probe() -> i64 { let a: i64 = 42; let c: bool = a > 0; let o: Option<&i64> = if c { Some(&a) } else { None }; match o { Some(r) => *r, None => 0 } }
