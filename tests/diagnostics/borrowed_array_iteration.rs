// expect: reject E_TRAIT
// pending: Explain that borrowed-array iteration lacks its implementation and suggest &values[..] to iterate the existing array as a slice instead of an associated-projection witness error.
pub fn probe() -> u64 { let values = [1u64, 2u64]; let mut sum = 0u64; for value in &values { sum += *value; } sum }
