mod values { pub fn answer() -> i64 { 42 } }
fn local() -> i64 { use values::answer; answer() }
pub fn probe() -> i64 { answer() }
