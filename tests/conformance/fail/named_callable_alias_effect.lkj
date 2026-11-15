// expect: reject E_BORROW
// witness: {"rustCode":"E0506"}
type View<'a> = &'a mut i64;
fn invalid<'a,F:FnMut(View<'a>)>(mut f:F,value:&'a mut i64){ f(value); *value=0; }
pub fn probe()->i64 {42}
