impl<'a> RecordInput<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], CoordinatorError> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(CoordinatorError::InvalidApplicationRegistry)?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or(CoordinatorError::InvalidApplicationRegistry)?;
        self.cursor = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], CoordinatorError> {
        self.take(N)?
            .try_into()
            .map_err(|_| CoordinatorError::InvalidApplicationRegistry)
    }

    fn u8(&mut self) -> Result<u8, CoordinatorError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, CoordinatorError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, CoordinatorError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn nonzero(&mut self) -> Result<u64, CoordinatorError> {
        let value = self.u64()?;
        if value == 0 {
            Err(CoordinatorError::InvalidApplicationRegistry)
        } else {
            Ok(value)
        }
    }

    fn boolean(&mut self) -> Result<bool, CoordinatorError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(CoordinatorError::InvalidApplicationRegistry),
        }
    }

    fn text(&mut self, maximum: usize) -> Result<String, CoordinatorError> {
        let length = usize::from(self.u16()?);
        if length == 0 || length > maximum {
            return Err(CoordinatorError::InvalidApplicationRegistry);
        }
        std::str::from_utf8(self.take(length)?)
            .map(str::to_owned)
            .map_err(|_| CoordinatorError::InvalidApplicationRegistry)
    }

    fn finish(&self) -> Result<(), CoordinatorError> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(CoordinatorError::InvalidApplicationRegistry)
        }
    }
}
