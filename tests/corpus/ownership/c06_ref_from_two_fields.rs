struct P { a: i64, b: i64 }
pub fn probe() -> i64 { let p = P { a: 42, b: 0 }; let c: bool = p.a > p.b; let r: &i64 = if c { &p.a } else { &p.b }; *r }
