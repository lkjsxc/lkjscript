use crate::Sha256;

pub(super) struct Encoder {
    state: Sha256,
}

impl Encoder {
    pub(super) fn new(domain: &[u8]) -> Self {
        let mut state = Sha256::new();
        state.update(domain);
        Self { state }
    }

    pub(super) fn tag(&mut self, value: u16) {
        self.fixed(&value.to_be_bytes());
    }

    pub(super) fn u8(&mut self, value: u8) {
        self.fixed(&[value]);
    }

    pub(super) fn u64(&mut self, value: u64) {
        self.fixed(&value.to_be_bytes());
    }

    pub(super) fn fixed(&mut self, value: &[u8]) {
        self.state.update(value);
    }

    pub(super) fn finish(self) -> [u8; 32] {
        self.state.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::Encoder;

    #[test]
    fn streams_more_than_former_descriptor_byte_boundary() {
        let chunk = [0xa5; 4 * 1024];
        let mut encoder = Encoder::new(&[]);
        for _ in 0..=(4 * 1024 * 1024) / chunk.len() {
            encoder.fixed(&chunk);
        }
        assert_ne!(encoder.finish(), [0; 32]);
    }
}
