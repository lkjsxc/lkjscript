// expect: reject E_PRIVACY
// witness: {"rustAccept":false,"rustCode":"E0603"}
mod hidden { enum Secret { Token } }
use hidden::Secret::*;
pub fn probe() -> i64 { 42 }
