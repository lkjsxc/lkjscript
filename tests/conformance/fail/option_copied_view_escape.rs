fn invalid<'a>() -> &'a i64 { let local = 42i64; let pointer = &local; Some(&pointer).copied().unwrap() }
pub fn probe() -> i64 { 42 }
