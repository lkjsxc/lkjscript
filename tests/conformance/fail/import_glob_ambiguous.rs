// expect: reject E_NAME
// witness: {"rustAccept":false,"rustCode":"E0659"}
mod first { pub fn read() -> i64 { 1 } }
mod second { pub fn read() -> i64 { 2 } }
use first::*;
use second::*;
pub fn probe() -> i64 { read() }
