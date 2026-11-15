// expect: reject E_TYPE
// rewrite: Name the missing closure input/result types at map_err and add an explicit std::String result annotation for this literal body.
pub fn probe() -> bool { String::from_utf8(Vec::new()).map_err(|_| "invalid".to_string()).is_ok() }
