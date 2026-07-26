use super::ImageCodecError;

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

    pub(super) fn finish(self) -> Vec<u8> {
        self.bytes
    }

    pub(super) fn u8(&mut self, value: u8) -> Result<(), ImageCodecError> {
        self.fixed(&[value])
    }

    pub(super) fn bool(&mut self, value: bool) -> Result<(), ImageCodecError> {
        self.u8(u8::from(value))
    }

    pub(super) fn u16(&mut self, value: u16) -> Result<(), ImageCodecError> {
        self.fixed(&value.to_be_bytes())
    }

    pub(super) fn u32(&mut self, value: u32) -> Result<(), ImageCodecError> {
        self.fixed(&value.to_be_bytes())
    }

    pub(super) fn i32(&mut self, value: i32) -> Result<(), ImageCodecError> {
        self.fixed(&value.to_be_bytes())
    }

    pub(super) fn u64(&mut self, value: u64) -> Result<(), ImageCodecError> {
        self.fixed(&value.to_be_bytes())
    }

    pub(super) fn fixed(&mut self, value: &[u8]) -> Result<(), ImageCodecError> {
        let next = self
            .bytes
            .len()
            .checked_add(value.len())
            .ok_or_else(|| ImageCodecError::new("image encoding byte overflow"))?;
        if next > self.maximum {
            return Err(ImageCodecError::new("image encoding byte limit exceeded"));
        }
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    pub(super) fn bytes(&mut self, value: &[u8]) -> Result<(), ImageCodecError> {
        let length = u64::try_from(value.len())
            .map_err(|_| ImageCodecError::new("image encoding length overflow"))?;
        self.u64(length)?;
        self.fixed(value)
    }

    pub(super) fn string(&mut self, value: &str) -> Result<(), ImageCodecError> {
        self.bytes(value.as_bytes())
    }

    pub(super) fn sequence<T>(
        &mut self,
        values: &[T],
        encode: impl Fn(&mut Self, &T) -> Result<(), ImageCodecError>,
    ) -> Result<(), ImageCodecError> {
        let count = u32::try_from(values.len())
            .map_err(|_| ImageCodecError::new("image encoding count overflow"))?;
        self.u32(count)?;
        for value in values {
            encode(self, value)?;
        }
        Ok(())
    }
}
