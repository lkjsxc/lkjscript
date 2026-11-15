// expect: reject E_PARSE
// Diagnostic: reject the raw byte literal at its prefix and suggest escaped byte-string syntax.
pub fn probe() { let bytes = br#"{"version":1}"#; assert!(bytes.len() > 0usize); }
