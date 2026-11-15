//! Byte operations used by readers. Every extent is a usize; there is no recursion.
use crate::Option;

pub fn starts_with(bytes: &[u8], prefix: &[u8]) -> bool {
    if prefix.len() > bytes.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < prefix.len() {
        if bytes[i] != prefix[i] {
            return false;
        }
        i = i + 1;
    }
    true
}

pub fn find_byte(bytes: &[u8], needle: u8) -> Option<usize> {
    let mut i: usize = 0;
    while i < bytes.len() {
        if bytes[i] == needle {
            return Option::Some(i);
        }
        i = i + 1;
    }
    Option::None
}

pub fn append_bytes(target: &mut Vec<u8>, source: &[u8]) {
    let mut i: usize = 0;
    while i < source.len() {
        target.push(source[i]);
        i = i + 1;
    }
}

// Validation uses the one source runtime through the checked text-view boundary.
pub fn valid_utf8(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok()
}
