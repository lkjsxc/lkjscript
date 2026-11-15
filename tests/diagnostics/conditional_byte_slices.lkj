// expect: reject E_TYPE
// rewrite: Convert unequal byte-string branches to shared slices at a declared slice argument, retaining one condition evaluation.
fn length(bytes: &[u8]) -> usize { bytes.len() }
pub fn probe() -> usize { length(if true { b"true" } else { b"false" }) }
