// expect: reject E_TYPE
// witness: {"rustAccept":false}
pub fn probe() -> i64 { let (first, .., .., last) = (1i64,2i64,3i64); first+last }
