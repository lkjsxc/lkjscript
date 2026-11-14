// expect: reject E_TYPE
// witness: {"rustAccept":false,"rustCode":"E0631"}
pub fn probe()->i64{let values=vec![1i64];let mut input=values.iter();input.any(|value:i64|value==1);42}
