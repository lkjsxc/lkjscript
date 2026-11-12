pub fn probe() -> i64 { let mut v: Vec<i64> = Vec::new(); v.push(2); v.push(40); let t: i64 = v[0]; v[0] = v[1]; v[1] = t; v[0] + v[1] }
