struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn extend(&mut self, bytes: &[u8]) -> io::Result<()> {
        let total = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| invalid("process frame length overflow"))?;
        if total > MAX_FRAME_BYTES {
            return Err(invalid("process frame exceeds bound"));
        }
        self.bytes
            .try_reserve(bytes.len())
            .map_err(|_| invalid("process frame allocation failed"))?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn u8(&mut self, value: u8) -> io::Result<()> {
        self.extend(&[value])
    }

    fn u32(&mut self, value: u32) -> io::Result<()> {
        self.extend(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> io::Result<()> {
        self.extend(&value.to_le_bytes())
    }

    fn usize(&mut self, value: usize) -> io::Result<()> {
        self.u64(u64::try_from(value).map_err(|_| invalid("usize exceeds protocol"))?)
    }

    fn bytes(&mut self, value: &[u8]) -> io::Result<()> {
        self.u64(u64::try_from(value.len()).map_err(|_| invalid("field exceeds protocol"))?)?;
        self.extend(value)
    }

    fn text(&mut self, value: &str, maximum: usize) -> io::Result<()> {
        if value.len() > maximum {
            return Err(invalid("text field exceeds bound"));
        }
        self.bytes(value.as_bytes())
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn take(&mut self, length: usize) -> io::Result<&'a [u8]> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or_else(|| invalid("process frame cursor overflow"))?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| invalid("truncated process frame"))?;
        self.cursor = end;
        Ok(value)
    }

    fn u8(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> io::Result<u32> {
        let bytes = self.take(4)?.try_into().map_err(|_| invalid("u32"))?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> io::Result<u64> {
        let bytes = self.take(8)?.try_into().map_err(|_| invalid("u64"))?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn usize(&mut self) -> io::Result<usize> {
        usize::try_from(self.u64()?).map_err(|_| invalid("protocol usize exceeds platform"))
    }

    fn bytes(&mut self, maximum: usize) -> io::Result<&'a [u8]> {
        let length = usize::try_from(self.u64()?)
            .map_err(|_| invalid("field length exceeds platform"))?;
        if length > maximum {
            return Err(invalid("process field exceeds bound"));
        }
        self.take(length)
    }

    fn text(&mut self, maximum: usize) -> io::Result<String> {
        let value = self.bytes(maximum)?;
        String::from_utf8(value.to_vec()).map_err(|_| invalid("process text is not UTF-8"))
    }

    fn finish(&self) -> io::Result<()> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(invalid("trailing process frame bytes"))
        }
    }
}

fn write_frame(output: &mut impl Write, body: Vec<u8>) -> io::Result<()> {
    let length = u64::try_from(body.len()).map_err(|_| invalid("process frame exceeds u64"))?;
    output.write_all(&length.to_le_bytes())?;
    output.write_all(&body)?;
    output.flush()
}

fn read_frame(input: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut length = [0_u8; 8];
    input.read_exact(&mut length)?;
    let length = usize::try_from(u64::from_le_bytes(length))
        .map_err(|_| invalid("process frame length exceeds platform"))?;
    if length > MAX_FRAME_BYTES {
        return Err(invalid("process frame exceeds bound"));
    }
    let mut body = Vec::new();
    body.try_reserve_exact(length)
        .map_err(|_| invalid("process frame allocation failed"))?;
    body.resize(length, 0);
    input.read_exact(&mut body)?;
    Ok(body)
}
