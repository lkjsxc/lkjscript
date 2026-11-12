struct Hs { v: Vec<i64>, n: i64 }
pub fn probe() -> i64 { let mut v: Vec<i64> = Vec::new(); v.push(1); let b: Box<Hs> = Box::new(Hs { v, n: 41 }); let h: Hs = *b; h.n + h.v.len() as i64 }
