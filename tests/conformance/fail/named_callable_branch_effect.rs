// expect: reject E_BORROW
// witness: {"rustCode":"E0506"}
fn invalid<'a,F:FnOnce(&'a i64)>(f:F,x:&'a mut i64,take:bool){if take{f(&*x);}*x=0;}
pub fn probe()->i64{42}
