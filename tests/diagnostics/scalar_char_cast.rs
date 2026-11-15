// expect: reject E_TYPE
// Diagnostic: suggest char::from_u32 with explicit handling of invalid Unicode scalars.
pub fn probe() -> char { let code = 0x10428u32; code as char }
