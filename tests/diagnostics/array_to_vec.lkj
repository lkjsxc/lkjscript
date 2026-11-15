// expect: reject E_UNSUPPORTED
// rewrite: Suggest `bytes[..].to_vec()` when to_vec is available on a slice but the receiver is a byte array.
pub fn probe() -> usize { b"HTTP".to_vec().len() }
