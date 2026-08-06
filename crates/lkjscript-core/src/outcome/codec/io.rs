pub(crate) struct Encoder {
    bytes: Vec<u8>,
    limits: ExecutionOutcomeCodecLimits,
}

impl Encoder {
    fn new(limits: ExecutionOutcomeCodecLimits) -> Self {
        Self {
            bytes: Vec::new(),
            limits,
        }
    }

    pub(crate) const fn structural_limits(&self) -> StructuralSnapshotLimits {
        self.limits.structural
    }

    fn reserve(&mut self, additional: usize) -> Result<()> {
        let total = self
            .bytes
            .len()
            .checked_add(additional)
            .ok_or_else(|| Error::msg("wire length overflow"))?;
        if total > self.limits.max_wire_bytes {
            return Err(Error::msg("wire value exceeds byte bound"));
        }
        self.bytes
            .try_reserve(additional)
            .map_err(|_| Error::msg("wire allocation failed"))
    }

    pub(crate) fn u8(&mut self, value: u8) -> Result<()> {
        self.reserve(1)?;
        self.bytes.push(value);
        Ok(())
    }

    pub(crate) fn u32(&mut self, value: u32) -> Result<()> {
        self.fixed(&value.to_le_bytes())
    }

    pub(crate) fn u64(&mut self, value: u64) -> Result<()> {
        self.fixed(&value.to_le_bytes())
    }

    pub(crate) fn i32(&mut self, value: i32) -> Result<()> {
        self.fixed(&value.to_le_bytes())
    }

    pub(crate) fn usize(&mut self, value: usize) -> Result<()> {
        self.u64(u64::try_from(value).map_err(|_| Error::msg("wire usize exceeds u64"))?)
    }

    pub(crate) fn fixed(&mut self, bytes: &[u8]) -> Result<()> {
        self.reserve(bytes.len())?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    pub(crate) fn bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.u32(u32::try_from(bytes.len()).map_err(|_| Error::msg("wire field exceeds u32"))?)?;
        self.fixed(bytes)
    }

    pub(crate) fn text(&mut self, text: &str) -> Result<()> {
        self.bytes(text.as_bytes())
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub(crate) struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
    limits: ExecutionOutcomeCodecLimits,
}

impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8], limits: ExecutionOutcomeCodecLimits) -> Result<Self> {
        if bytes.len() > limits.max_wire_bytes {
            return Err(Error::msg("wire value exceeds byte bound"));
        }
        Ok(Self {
            bytes,
            cursor: 0,
            limits,
        })
    }

    pub(crate) const fn structural_limits(&self) -> StructuralSnapshotLimits {
        self.limits.structural
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or_else(|| Error::msg("wire cursor overflow"))?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| Error::msg("truncated wire value"))?;
        self.cursor = end;
        Ok(value)
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().map_err(|_| Error::msg("u32"))?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().map_err(|_| Error::msg("u64"))?))
    }

    pub(crate) fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().map_err(|_| Error::msg("i32"))?))
    }

    pub(crate) fn usize(&mut self) -> Result<usize> {
        usize::try_from(self.u64()?).map_err(|_| Error::msg("wire usize exceeds platform"))
    }

    pub(crate) fn bytes(&mut self) -> Result<&'a [u8]> {
        let length = usize::try_from(self.u32()?).map_err(|_| Error::msg("wire field length"))?;
        self.take(length)
    }

    pub(crate) fn text(&mut self) -> Result<String> {
        String::from_utf8(self.bytes()?.to_vec()).map_err(|_| Error::msg("wire text is not UTF-8"))
    }

    fn finish(&self) -> Result<()> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::msg("trailing wire bytes"))
        }
    }
}
