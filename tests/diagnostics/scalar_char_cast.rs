// expect: reject E_TYPE
// pending: Explain that u32-to-char casts are unavailable and suggest char::from_u32(code) with its invalid-scalar result, instead of reporting an invalid intrinsic/signature.
pub fn probe() -> char { let code = 0x10428u32; code as char }
