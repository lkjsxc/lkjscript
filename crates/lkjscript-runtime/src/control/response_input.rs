struct ResponseInput<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> ResponseInput<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ControlError> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(ControlError::Oversized)?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or(ControlError::Malformed("truncated response"))?;
        self.cursor = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], ControlError> {
        self.take(N)?
            .try_into()
            .map_err(|_| ControlError::Malformed("response field width"))
    }

    fn u8(&mut self) -> Result<u8, ControlError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ControlError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, ControlError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, ControlError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn nonzero(&mut self) -> Result<u64, ControlError> {
        let value = self.u64()?;
        if value == 0 {
            Err(ControlError::InvalidIdentity)
        } else {
            Ok(value)
        }
    }

    fn boolean(&mut self) -> Result<bool, ControlError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(ControlError::Malformed("response boolean")),
        }
    }

    fn optional_nonzero(&mut self) -> Result<Option<u64>, ControlError> {
        match self.u8()? {
            0 => Ok(None),
            1 => self.nonzero().map(Some),
            _ => Err(ControlError::Malformed("response identity option")),
        }
    }

    fn text(&mut self, maximum: usize) -> Result<String, ControlError> {
        let length = usize::from(self.u16()?);
        if length == 0 || length > maximum {
            return Err(ControlError::Malformed("response text length"));
        }
        std::str::from_utf8(self.take(length)?)
            .map(str::to_owned)
            .map_err(|_| ControlError::Malformed("response text UTF-8"))
    }

    fn finish(&self) -> Result<(), ControlError> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(ControlError::Malformed("trailing response bytes"))
        }
    }
}
