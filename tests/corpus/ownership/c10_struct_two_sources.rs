struct Two<'a> { x: &'a i64, y: &'a i64 }
pub fn probe() -> i64 { let a: i64 = 40; let b: i64 = 2; let t = Two { x: &a, y: &b }; *t.x + *t.y }
