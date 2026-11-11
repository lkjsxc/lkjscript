fn consume(value: Box<i64>) -> i64 { *value }
pub fn probe() -> i64 { consume(Box::new(42i64)) }
