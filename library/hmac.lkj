//! HMAC-SHA256 using the shared source hash, per RFC 2104 section 2.
//! https://www.rfc-editor.org/rfc/rfc2104.html#section-2
//! The SHA-256 input-length representation applies, including the inner pad.
use crate::sha256::Sha256;

pub fn sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut block = [0u8; 64];
    if key.len() > block.len() {
        let digest = crate::sha256::digest(key);
        block[..32].copy_from_slice(&digest);
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut inner_pad = block;
    let mut outer_pad = block;
    for index in 0usize..64usize {
        inner_pad[index] ^= 0x36u8;
        outer_pad[index] ^= 0x5cu8;
    }
    let mut inner = Sha256::new();
    inner.update(&inner_pad);
    inner.update(message);
    let digest = inner.finish();
    let mut outer = Sha256::new();
    outer.update(&outer_pad);
    outer.update(&digest);
    outer.finish()
}

/// Length is public. For equal lengths, every byte participates without a
/// content-dependent branch or memory index. This source property must also be
/// checked in emitted code after compiler/optimization changes; it is not an
/// assertion that all possible future compilers preserve constant-time behavior.
pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() { return false; }
    let mut difference = 0u8;
    let mut index = 0usize;
    while index < left.len() {
        difference |= left[index] ^ right[index];
        index += 1usize;
    }
    difference == 0u8
}
