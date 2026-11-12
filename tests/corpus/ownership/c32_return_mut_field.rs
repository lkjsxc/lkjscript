struct S { a: Vec<i64> }
fn field(s: &mut S) -> &mut Vec<i64> { &mut s.a }
pub fn probe() -> i64 { let mut s = S { a: Vec::new() }; field(&mut s).push(42); s.a[0] }
