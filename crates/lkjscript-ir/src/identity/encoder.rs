use crate::{IrError, Result};
use lkjscript_core::Sha256;

pub(super) struct Encoder {
    state: Sha256,
    error: Option<IrError>,
}

impl Encoder {
    pub(super) fn new(domain: &[u8]) -> Self {
        let mut encoder = Self {
            state: Sha256::new(),
            error: None,
        };
        encoder.bytes(domain);
        encoder
    }

    fn append(&mut self, value: &[u8]) {
        if self.error.is_none() {
            self.state.update(value);
        }
    }

    pub(super) fn fail(&mut self, message: &'static str) {
        if self.error.is_none() {
            self.error = Some(IrError::new(message));
        }
    }
    pub(super) fn tag(&mut self, value: u16) {
        self.append(&value.to_be_bytes());
    }
    pub(super) fn bool(&mut self, value: bool) {
        self.append(&[u8::from(value)]);
    }
    pub(super) fn u16(&mut self, value: u16) {
        self.append(&value.to_be_bytes());
    }
    pub(super) fn wide(&mut self, value: impl Into<u64>) {
        self.append(&value.into().to_be_bytes());
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
            Err(_) => self.fail("verified SSA identity length overflow"),
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
            None => Ok(self.state.finish()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Encoder;

    #[test]
    #[ignore = "release stress geometry for the former append-count ceiling"]
    fn more_than_former_append_count_boundary_succeeds() {
        let mut encoder = Encoder::new(&[]);
        for _ in 0..=8_388_608 {
            encoder.fixed(&[0]);
        }
        assert!(matches!(encoder.finish(), Ok(digest) if digest != [0; 32]));
    }
}
