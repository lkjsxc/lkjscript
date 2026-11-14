// expect: reject E_NAME
// witness: {"rustAccept":false,"rustCode":"E0425"}
mod values { pub fn answer() -> i64 { 42 } }
fn local() -> i64 { use values::answer; answer() }
pub fn probe() -> i64 { answer() }
