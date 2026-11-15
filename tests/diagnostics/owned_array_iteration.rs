// expect: reject E_TRAIT
// diagnostic edits: On failed IntoIterator resolution, turn this owned literal array into a Vec.
pub fn probe() -> u64 { let mut sum = 0u64; for value in [1u64, 2u64] { sum += value; } sum }
