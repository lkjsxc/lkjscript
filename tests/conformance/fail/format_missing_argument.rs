// expect: reject E_FORMAT
// witness: {"rustAccept":false}
pub fn probe()->i64 {let x=42;format!("{x}{0}").len() as i64}
