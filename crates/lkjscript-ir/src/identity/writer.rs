use super::IdentityError;

pub(super) struct Writer {
    bytes: Vec<u8>,
    maximum: usize,
}

impl Writer {
    pub(super) fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
        }
    }

    pub(super) fn finish(&self) -> &[u8] {
        &self.bytes
    }

    pub(super) fn u8(&mut self, value: u8) -> Result<(), IdentityError> {
        self.extend(&[value])
    }

    pub(super) fn bool(&mut self, value: bool) -> Result<(), IdentityError> {
        self.u8(u8::from(value))
    }

    pub(super) fn u16(&mut self, value: u16) -> Result<(), IdentityError> {
        self.extend(&value.to_be_bytes())
    }

    pub(super) fn u32(&mut self, value: u32) -> Result<(), IdentityError> {
        self.extend(&value.to_be_bytes())
    }

    pub(super) fn u64(&mut self, value: u64) -> Result<(), IdentityError> {
        self.extend(&value.to_be_bytes())
    }

    pub(super) fn i64(&mut self, value: i64) -> Result<(), IdentityError> {
        self.extend(&value.to_be_bytes())
    }

    pub(super) fn fixed(&mut self, value: &[u8]) -> Result<(), IdentityError> {
        self.extend(value)
    }

    pub(super) fn bytes(&mut self, value: &[u8]) -> Result<(), IdentityError> {
        let length = u64::try_from(value.len()).map_err(|_| IdentityError("identity length"))?;
        self.extend(&length.to_be_bytes())?;
        self.extend(value)
    }

    pub(super) fn string(&mut self, value: &str) -> Result<(), IdentityError> {
        self.bytes(value.as_bytes())
    }

    pub(super) fn sequence<T>(
        &mut self,
        values: &[T],
        encode: impl Fn(&mut Self, &T) -> Result<(), IdentityError>,
    ) -> Result<(), IdentityError> {
        let count = u32::try_from(values.len()).map_err(|_| IdentityError("identity count"))?;
        self.u32(count)?;
        for value in values {
            encode(self, value)?;
        }
        Ok(())
    }

    pub(super) fn option<T>(
        &mut self,
        value: Option<&T>,
        encode: impl Fn(&mut Self, &T) -> Result<(), IdentityError>,
    ) -> Result<(), IdentityError> {
        match value {
            Some(value) => {
                self.u8(1)?;
                encode(self, value)
            }
            None => self.u8(0),
        }
    }

    fn extend(&mut self, value: &[u8]) -> Result<(), IdentityError> {
        let next = self
            .bytes
            .len()
            .checked_add(value.len())
            .ok_or(IdentityError("identity byte overflow"))?;
        if next > self.maximum {
            return Err(IdentityError("identity byte limit exceeded"));
        }
        self.bytes.extend_from_slice(value);
        Ok(())
    }
}
