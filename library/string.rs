//! UTF-8 strings own bytes. Validation happens when unchecked bytes become a String.
use crate::Result;
use crate::bytes::valid_utf8;

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

impl FromUtf8Error {
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}
