struct Value { value: Box<i64> }
pub fn probe() -> i64 { let value = Value { value: Box::new(42) }; let copied = Some(&value).copied().unwrap(); *copied.value }
