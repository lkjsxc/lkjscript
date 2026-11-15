// expect: reject E_BORROW
// entry: probe
// witness: {"rustCode":"E0597"}
pub fn probe()->i64{let result:&i64;{let short=42i64;result=Some(&short).map(|value:&i64|value).unwrap();}*result}
