// expect: reject E_TYPE
// witness: {"rustAccept":false}
fn invalid() { let mut values = Vec::new(); values.push(1i64); values.push(true); }
