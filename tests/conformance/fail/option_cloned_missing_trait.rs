struct Value { value: i64 }
pub fn probe() -> i64 { let value = Value { value: 42 }; Some(&value).cloned().unwrap().value }
