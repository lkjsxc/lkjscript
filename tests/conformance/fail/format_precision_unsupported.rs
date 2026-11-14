// expect: reject E_UNSUPPORTED
pub fn probe()->i64 {let value=format!("{:.2}","hello");value.len() as i64}
