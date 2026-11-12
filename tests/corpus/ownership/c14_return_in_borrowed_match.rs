enum E { A(Vec<i64>), B }
fn f(e: &E) -> i64 { match e { E::A(v) => { if v.len() > 0usize { return v[0]; } 0 } E::B => 1 } }
pub fn probe() -> i64 { let mut v: Vec<i64> = Vec::new(); v.push(42); f(&E::A(v)) }
