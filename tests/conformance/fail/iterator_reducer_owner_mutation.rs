// expect: reject E_BORROW
// witness: {"rustAccept":false,"rustCode":"E0502"}
pub fn probe()->i64{let mut values=vec![1i64];let mut input=values.iter();input.any(|_|{values.push(2);true});42}
