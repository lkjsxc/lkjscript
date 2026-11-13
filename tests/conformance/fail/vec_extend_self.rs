pub fn probe() -> i64 { let mut values = vec![21i64]; values.extend_from_slice(&values[..]); values[0] + values[1] }
