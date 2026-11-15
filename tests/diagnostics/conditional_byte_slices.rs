// expect: reject E_TYPE
// pending: Explain that these byte-string branches have different array lengths; suggest converting both branches to &[u8] before passing the conditional to a slice parameter.
fn length(bytes: &[u8]) -> usize { bytes.len() }
pub fn probe() -> usize { length(if true { b"true" } else { b"false" }) }
