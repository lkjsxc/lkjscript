// expect: reject E_TRAIT
// pending: Explain that owned-array iteration lacks its implementation and suggest indexing the array or iterating a Vec instead of an associated-projection witness error.
pub fn probe() -> u64 { let mut sum = 0u64; for value in [1u64, 2u64] { sum += value; } sum }
