// expect: reject E_TRAIT
// rewrite: Borrowed-array iteration needs an applicable IntoIterator implementation; use &values[..] to borrow the existing array as a shared slice, preserving evaluation and cleanup.
pub fn probe() -> u64 { let values = [1u64, 2u64]; let mut sum = 0u64; for value in &values { sum += *value; } sum }
