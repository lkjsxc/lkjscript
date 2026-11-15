// expect: reject E_UNSUPPORTED
// pending: Suggest `bytes[..].copy_from_slice(source)` when copy_from_slice is available on slices but the receiver is an array.
pub fn probe() -> u8 { let mut bytes = [0u8; 2]; bytes.copy_from_slice(b"ok"); bytes[0] }
