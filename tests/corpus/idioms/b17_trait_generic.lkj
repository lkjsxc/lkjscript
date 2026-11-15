trait Shape { fn area(&self) -> i64; }
struct Sq { s: i64 }
impl Shape for Sq { fn area(&self) -> i64 { self.s * self.s } }
fn total<T: Shape>(xs: &[T]) -> i64 { let mut s: i64 = 0; let mut i: usize = 0; while i < xs.len() { s += xs[i].area(); i += 1; } s }
pub fn probe() -> i64 { let v = vec![Sq { s: 4 }, Sq { s: 5 }, Sq { s: 1 }]; total(&v) }
