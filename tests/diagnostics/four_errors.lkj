fn a() -> i64 { let x: i64 = true; x }
fn b() -> i64 { let v: Vec<i64> = Vec::new(); let r: &i64 = &v[0]; let w: Vec<i64> = v; *r }
fn c2() -> i64 { undefined_name + 1 }
fn d() -> i64 { let s: Box<i64> = Box::new(1); let t: Box<i64> = s; *s }
pub fn probe() -> i64 { a() + b() + c2() + d() }
