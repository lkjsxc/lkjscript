// expect: reject E_TYPE
// witness: {"rustAccept":false,"rustCode":"E0308"}
pub fn probe()->i64{let values=vec![1i64];let mut input=values.iter();input.all(|_|42i64);42}
