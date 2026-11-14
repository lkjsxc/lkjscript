// expect: reject E_TYPE
// witness: {"rustAccept":false,"rustCode":"E0220"}
fn invalid<I,F:Fn(I::Item)->i64>(value:I,function:F)->i64 { 42 }
pub fn probe()->i64 { 42 }
