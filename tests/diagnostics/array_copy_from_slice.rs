// expect: reject E_UNSUPPORTED
// rewrite: Use a mutable slice receiver for array copy_from_slice; preserve the receiver evaluation and require Copy elements.
pub fn probe() -> u8 { let mut bytes = [0u8; 2]; bytes.copy_from_slice(b"ok"); bytes[0] }
