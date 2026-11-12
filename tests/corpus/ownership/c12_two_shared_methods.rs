struct S { v: Vec<i64> }
impl S { fn first(&self) -> &i64 { &self.v[0] } fn n(&self) -> i64 { self.v.len() as i64 } }
pub fn probe() -> i64 { let mut v: Vec<i64> = Vec::new(); v.push(41); let s = S { v }; let f = s.first(); let n = s.n(); *f + n }
