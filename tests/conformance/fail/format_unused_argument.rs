// expect: reject E_FORMAT
// witness: {"rustAccept":false}
pub fn probe()->i64 {format!("constant",42).len() as i64}
