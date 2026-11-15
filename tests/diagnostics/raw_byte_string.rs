// expect: reject E_PARSE
// pending: Explain that raw byte-string literals are unsupported and suggest a byte string with escaped quotes instead of reporting a missing semicolon and unrelated identifier errors.
pub fn probe() { let bytes = br#"{"version":1}"#; assert!(bytes.len() > 0usize); }
