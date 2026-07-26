use super::ImageCodecError;

pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
    records: u64,
    maximum_records: u64,
}

impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8], maximum_records: u64) -> Self {
        Self {
            bytes,
            offset: 0,
            records: 0,
            maximum_records,
        }
    }

    pub(super) fn done(&self) -> bool {
        self.offset == self.bytes.len()
    }

    pub(super) fn u8(&mut self) -> Result<u8, ImageCodecError> {
        Ok(self.take(1)?[0])
    }

    pub(super) fn bool(&mut self) -> Result<bool, ImageCodecError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(ImageCodecError::new("noncanonical image boolean")),
        }
    }

    pub(super) fn u16(&mut self) -> Result<u16, ImageCodecError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    pub(super) fn u32(&mut self) -> Result<u32, ImageCodecError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    pub(super) fn i32(&mut self) -> Result<i32, ImageCodecError> {
        Ok(i32::from_be_bytes(self.array()?))
    }

    pub(super) fn u64(&mut self) -> Result<u64, ImageCodecError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    pub(super) fn fixed<const N: usize>(&mut self) -> Result<[u8; N], ImageCodecError> {
        self.array()
    }

    pub(super) fn bytes(&mut self) -> Result<&'a [u8], ImageCodecError> {
        let length = usize::try_from(self.u64()?)
            .map_err(|_| ImageCodecError::new("image byte length overflow"))?;
        self.take(length)
    }

    pub(super) fn string(&mut self) -> Result<String, ImageCodecError> {
        let text = std::str::from_utf8(self.bytes()?)
            .map_err(|_| ImageCodecError::new("image string is not UTF-8"))?;
        Ok(text.to_owned())
    }

    pub(super) fn count(&mut self) -> Result<usize, ImageCodecError> {
        let count = u64::from(self.u32()?);
        self.records = self
            .records
            .checked_add(count)
            .ok_or_else(|| ImageCodecError::new("image record count overflow"))?;
        if self.records > self.maximum_records {
            return Err(ImageCodecError::new("image record limit exceeded"));
        }
        usize::try_from(count).map_err(|_| ImageCodecError::new("image record count overflow"))
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], ImageCodecError> {
        self.take(N)?
            .try_into()
            .map_err(|_| ImageCodecError::new("truncated image"))
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ImageCodecError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| ImageCodecError::new("image offset overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| ImageCodecError::new("truncated image"))?;
        self.offset = end;
        Ok(value)
    }
}
