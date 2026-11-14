// expect: reject E_TYPE
// entry: probe
// witness: {"rustCode":"E0631"}
pub fn probe()->i64{let value=Some(42i64);value.map(|x:bool|if x{42i64}else{0i64}).unwrap()}
