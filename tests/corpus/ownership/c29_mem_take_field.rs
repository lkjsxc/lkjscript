struct S { v: Vec<i64> }
impl S { fn drain(&mut self) -> Vec<i64> { std::mem::replace(&mut self.v, Vec::new()) } }
pub fn probe() -> i64 { let mut v: Vec<i64> = Vec::new(); v.push(42); let mut s = S { v }; let d = s.drain(); d[0] + s.v.len() as i64 }
