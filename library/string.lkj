//! UTF-8 strings own bytes. Validation happens when unchecked bytes become a String.
use crate::Result;
use crate::bytes::{append_bytes, starts_with, valid_utf8};

#[derive(PartialOrd, Ord)]
pub struct String {
    bytes: Vec<u8>,
}

pub struct FromUtf8Error {
    bytes: Vec<u8>,
}

impl String {
    pub fn new() -> String {
        String { bytes: Vec::new() }
    }

    pub fn from_utf8(bytes: Vec<u8>) -> Result<String, FromUtf8Error> {
        if valid_utf8(&bytes) {
            Result::Ok(String { bytes })
        } else {
            Result::Err(FromUtf8Error { bytes })
        }
    }

    pub fn from(value: &str) -> String {
        String::from_str(value)
    }

    pub fn from_str(value: &str) -> String {
        let mut result = String::new();
        result.push_str(value);
        result
    }

    pub fn push_str(&mut self, value: &str) {
        append_bytes(&mut self.bytes, value.as_bytes());
    }

    pub fn push(&mut self, value: char) {
        let code = value as u32;
        if code < 0x80u32 {
            self.bytes.push(code as u8);
        } else if code < 0x800u32 {
            self.bytes.push(((code >> 6) | 0xc0u32) as u8);
            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);
        } else if code < 0x10000u32 {
            self.bytes.push(((code >> 12) | 0xe0u32) as u8);
            self.bytes.push((((code >> 6) & 0x3fu32) | 0x80u32) as u8);
            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);
        } else {
            self.bytes.push(((code >> 18) | 0xf0u32) as u8);
            self.bytes.push((((code >> 12) & 0x3fu32) | 0x80u32) as u8);
            self.bytes.push((((code >> 6) & 0x3fu32) | 0x80u32) as u8);
            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);
        }
    }

    pub fn starts_with<P: crate::str::Pattern>(&self, pattern: P) -> bool {
        crate::str::__lkjscript_starts_with(self.as_str(), pattern)
    }
    pub fn ends_with<P: crate::str::Pattern>(&self, pattern: P) -> bool {
        crate::str::__lkjscript_ends_with(self.as_str(), pattern)
    }
    pub fn contains<P: crate::str::Pattern>(&self, pattern: P) -> bool {
        crate::str::__lkjscript_contains(self.as_str(), pattern)
    }
    pub fn find<P: crate::str::Pattern>(&self, pattern: P) -> crate::Option<usize> {
        crate::str::__lkjscript_find(self.as_str(), pattern)
    }
    pub fn rfind<P: crate::str::Pattern>(&self, pattern: P) -> crate::Option<usize> {
        crate::str::__lkjscript_rfind(self.as_str(), pattern)
    }
    pub fn is_ascii(&self) -> bool {
        crate::str::__lkjscript_is_ascii(self.as_str())
    }
    pub fn to_ascii_lowercase(&self) -> String {
        crate::str::__lkjscript_to_ascii_lowercase(self.as_str())
    }
    pub fn to_ascii_uppercase(&self) -> String {
        crate::str::__lkjscript_to_ascii_uppercase(self.as_str())
    }
    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {
        crate::str::__lkjscript_eq_ignore_ascii_case(self.as_str(), other)
    }
    pub fn repeat(&self, count: usize) -> String {
        crate::str::__lkjscript_repeat(self.as_str(), count)
    }
    pub fn replace<P: crate::str::Pattern>(&self, pattern: P, replacement: &str) -> String {
        crate::str::__lkjscript_replace(self.as_str(), pattern, replacement)
    }

    pub fn to_string(&self) -> String {
        self.clone()
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(self.bytes.as_slice()).unwrap()
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.len() == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn clear(&mut self) {
        while self.bytes.len() > 0 {
            self.bytes.pop();
        }
    }
}

// Only valid UTF-8 can enter the private byte storage, so exact copies preserve it.
impl Clone for String {
    fn clone(&self) -> Self {
        let mut bytes: Vec<u8> = Vec::new();
        append_bytes(&mut bytes, self.bytes.as_slice());
        String { bytes }
    }

    fn clone_from(&mut self, source: &Self) {
        self.clear();
        append_bytes(&mut self.bytes, source.bytes.as_slice());
    }
}

impl PartialEq for String {
    fn eq(&self, other: &Self) -> bool {
        self.bytes.len() == other.bytes.len()
            && starts_with(self.bytes.as_slice(), other.bytes.as_slice())
    }

    // lkjscript currently requires an executable method for each builtin operation.
    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl Eq for String {}

impl FromUtf8Error {
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

// Resolved compiler helpers retain the same source String implementation.
pub fn __lkjscript_new() -> String {
    String::new()
}

// Range syntax resolves this declaration, preserving the source String invariant.
pub fn __lkjscript_as_str(value: &String) -> &str {
    value.as_str()
}

/// Canonical lexical String ordering uses its UTF-8 bytes, as Rust does.
pub fn __lkjscript_compare(left: &String, right: &String) -> i32 {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut index = 0usize;
    while index < left.len() && index < right.len() {
        if left[index] < right[index] {
            return -1;
        }
        if left[index] > right[index] {
            return 1;
        }
        index = index + 1;
    }
    if left.len() < right.len() {
        -1
    } else if left.len() > right.len() {
        1
    } else {
        0
    }
}
