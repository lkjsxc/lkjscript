use crate::{Error, Result};

const MAX_CANONICAL_BYTES: usize = 64 * 1024 * 1024;
const MAX_CANONICAL_WORK: usize = 8 * 1024 * 1024;

pub(super) struct Encoder {
    bytes: Vec<u8>,
    work: usize,
    error: Option<Error>,
}

impl Encoder {
    pub(super) fn new(domain: &[u8]) -> Self {
        let mut out = Self {
            bytes: Vec::new(),
            work: 0,
            error: None,
        };
        out.bytes(domain);
        out
    }
    fn append(&mut self, value: &[u8]) {
        if self.error.is_some() {
            return;
        }
        let Some(size) = self.bytes.len().checked_add(value.len()) else {
            self.fail("validated bytecode identity size overflow");
            return;
        };
        let Some(work) = self.work.checked_add(1) else {
            self.fail("validated bytecode identity work overflow");
            return;
        };
        if size > MAX_CANONICAL_BYTES || work > MAX_CANONICAL_WORK {
            self.fail("validated bytecode identity bound exceeded");
            return;
        }
        if self.bytes.try_reserve(value.len()).is_err() {
            self.fail("validated bytecode identity allocation failed");
            return;
        }
        self.bytes.extend_from_slice(value);
        self.work = work;
    }
    pub(super) fn fail(&mut self, message: &'static str) {
        if self.error.is_none() {
            self.error = Some(Error::msg(message));
        }
    }
    pub(super) fn tag(&mut self, value: u16) {
        self.append(&value.to_be_bytes());
    }
    pub(super) fn bool(&mut self, value: bool) {
        self.append(&[u8::from(value)]);
    }
    pub(super) fn u8(&mut self, value: u8) {
        self.append(&[value]);
    }
    pub(super) fn u16(&mut self, value: u16) {
        self.append(&value.to_be_bytes());
    }
    pub(super) fn u32(&mut self, value: u32) {
        self.append(&value.to_be_bytes());
    }
    pub(super) fn u64(&mut self, value: u64) {
        self.append(&value.to_be_bytes());
    }
    pub(super) fn i64(&mut self, value: i64) {
        self.append(&value.to_be_bytes());
    }
    pub(super) fn len(&mut self, value: usize) {
        match u64::try_from(value) {
            Ok(value) => self.u64(value),
            Err(_) => self.fail("validated bytecode identity length overflow"),
        }
    }
    pub(super) fn offset(&mut self, value: usize) {
        match u64::try_from(value) {
            Ok(value) => self.u64(value),
            Err(_) => self.fail("decoded bytecode offset overflow"),
        }
    }
    pub(super) fn bytes(&mut self, value: &[u8]) {
        self.len(value.len());
        self.append(value);
    }
    pub(super) fn fixed(&mut self, value: &[u8]) {
        self.append(value);
    }
    pub(super) fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }
    pub(super) fn option<T>(&mut self, value: Option<&T>, encode: impl FnOnce(&mut Self, &T)) {
        self.bool(value.is_some());
        if let Some(value) = value {
            encode(self, value);
        }
    }
    pub(super) fn sequence<T>(&mut self, values: &[T], mut encode: impl FnMut(&mut Self, &T)) {
        self.len(values.len());
        for value in values {
            encode(self, value);
        }
    }
    pub(super) fn finish(self) -> Result<[u8; 32]> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(crate::sha256(&self.bytes)),
        }
    }
}
