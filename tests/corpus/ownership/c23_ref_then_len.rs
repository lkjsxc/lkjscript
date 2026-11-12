pub fn probe() -> i64 { let mut v: Vec<i64> = Vec::new(); v.push(41); let first = &v[0]; let n = v.len(); *first + n as i64 }
