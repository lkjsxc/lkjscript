//! UTF-8 strings own bytes. Validation happens when unchecked bytes become a String.
use crate::Result;
use crate::bytes::{append_bytes, starts_with, valid_utf8};

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

    pub fn from_str(value: &str) -> String {
        let mut result = String::new();
        result.push_str(value);
        result
    }

    pub fn push_str(&mut self, value: &str) {
        append_bytes(&mut self.bytes, value.as_bytes());
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
