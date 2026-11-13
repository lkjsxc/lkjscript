// Rust accepts; automatic array-to-slice method-receiver coercion is not implemented.
pub fn probe() -> i64 { let values = [20i64, 22]; if values.iter().any(|item| *item == 22) { 42 } else { 0 } }
