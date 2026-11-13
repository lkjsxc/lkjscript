pub fn probe() -> i64 { let mut values = vec![20i64]; let view = &values[0]; values.extend_from_slice(&[22i64]); *view + values[1] }
