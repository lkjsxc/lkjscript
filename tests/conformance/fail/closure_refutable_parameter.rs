pub fn probe()->i64 { let f=|Some(value):Option<i64>|value; f(Some(42)) }
