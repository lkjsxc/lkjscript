struct P { a: i64, b: i64 }
pub fn probe() -> i64 { let mut p = P { a: 40, b: 2 }; let x = &mut p.a; let y = &mut p.b; *x = *x + *y; p.a }
