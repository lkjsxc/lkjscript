fn consume(v: Vec<i64>) -> i64 { v.len() as i64 }
 enum E { A(Vec<i64>), B(i64) }
pub fn probe() -> i64 { let mut v: Vec<i64> = Vec::new(); v.push(1); let e = E::A(v); match e { E::A(w) => 41 + consume(w), E::B(n) => n } }
