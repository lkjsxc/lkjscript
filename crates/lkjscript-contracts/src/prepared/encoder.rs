use super::{PreparedProgramError, MAX_PREPARED_DESCRIPTOR_BYTES};

pub(super) struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    pub(super) fn new(domain: &[u8]) -> Result<Self, PreparedProgramError> {
        let mut value = Self { bytes: Vec::new() };
        value.fixed(domain)?;
        Ok(value)
    }

    pub(super) fn tag(&mut self, value: u16) -> Result<(), PreparedProgramError> {
        self.fixed(&value.to_be_bytes())
    }

    pub(super) fn u8(&mut self, value: u8) -> Result<(), PreparedProgramError> {
        self.fixed(&[value])
    }

    pub(super) fn u64(&mut self, value: u64) -> Result<(), PreparedProgramError> {
        self.fixed(&value.to_be_bytes())
    }

    pub(super) fn fixed(&mut self, value: &[u8]) -> Result<(), PreparedProgramError> {
        let length = self
            .bytes
            .len()
            .checked_add(value.len())
            .ok_or(PreparedProgramError::BoundExceeded)?;
        if length > MAX_PREPARED_DESCRIPTOR_BYTES {
            return Err(PreparedProgramError::BoundExceeded);
        }
        self.bytes
            .try_reserve(value.len())
            .map_err(|_| PreparedProgramError::AllocationFailed)?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    pub(super) fn finish(self) -> [u8; 32] {
        crate::sha256(&self.bytes)
    }
}
