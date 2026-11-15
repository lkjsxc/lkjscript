// expect: reject E_BORROW
// witness: {"rustAccept":false,"rustCode":"E0499"}
pub fn probe()->i64{let values=vec![1i64];let mut input=values.iter();input.any(|_|{input.next();true});42}
