// expect: reject E_TYPE
// pending: Name the missing closure input/result types at map_err and suggest an explicit match when inference cannot supply them.
pub fn probe() -> bool { String::from_utf8(Vec::new()).map_err(|_| "invalid".to_string()).is_ok() }
