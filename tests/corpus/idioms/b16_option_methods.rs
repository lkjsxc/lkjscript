pub fn probe() -> i64 { let o: Option<i64> = Some(41); let p = o.map(|x| x + 1); p.unwrap_or(0) }
