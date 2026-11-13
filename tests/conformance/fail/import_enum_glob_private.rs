mod hidden { enum Secret { Token } }
use hidden::Secret::*;
pub fn probe() -> i64 { 42 }
