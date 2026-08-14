use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TagDomain {
    Change,
    Error,
    Node,
    NodeTarget,
    Operation,
    ProtocolMessage,
    RuntimeValue,
    SemanticType,
    TransactionOperation,
    Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CodecErrorKind {
    InvalidBoolean(u8),
    InvalidUtf8,
    LengthOverflow,
    PolicyExceeded,
    TrailingBytes,
    UnexpectedEnd,
    UnknownTag { domain: TagDomain, tag: u8 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CodecError {
    pub offset: usize,
    pub kind: CodecErrorKind,
}

impl fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "binary decoding failed at byte {}: {:?}",
            self.offset, self.kind
        )
    }
}

pub(crate) struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    pub(crate) fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(crate) fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    pub(crate) fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn fixed(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    pub(crate) fn bytes(&mut self, value: &[u8]) -> Result<(), CodecError> {
        let length = u64::try_from(value.len()).map_err(|_| CodecError {
            offset: self.bytes.len(),
            kind: CodecErrorKind::LengthOverflow,
        })?;
        self.u64(length);
        self.fixed(value);
        Ok(())
    }

    pub(crate) fn string(&mut self, value: &str) -> Result<(), CodecError> {
        self.bytes(value.as_bytes())
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub(crate) fn position(&self) -> usize {
        self.position
    }

    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, CodecError> {
        let value = *self
            .bytes
            .get(self.position)
            .ok_or_else(|| self.error(CodecErrorKind::UnexpectedEnd))?;
        self.position += 1;
        Ok(value)
    }

    pub(crate) fn bool(&mut self) -> Result<bool, CodecError> {
        let offset = self.position;
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(CodecError {
                offset,
                kind: CodecErrorKind::InvalidBoolean(value),
            }),
        }
    }

    pub(crate) fn u16(&mut self) -> Result<u16, CodecError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, CodecError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, CodecError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub(crate) fn i64(&mut self) -> Result<i64, CodecError> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    pub(crate) fn fixed(&mut self, length: usize) -> Result<&'a [u8], CodecError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| self.error(CodecErrorKind::LengthOverflow))?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| self.error(CodecErrorKind::UnexpectedEnd))?;
        self.position = end;
        Ok(value)
    }

    pub(crate) fn bytes(&mut self, maximum: usize) -> Result<&'a [u8], CodecError> {
        let length_offset = self.position;
        let encoded = self.u64()?;
        let length = usize::try_from(encoded).map_err(|_| CodecError {
            offset: length_offset,
            kind: CodecErrorKind::LengthOverflow,
        })?;
        if length > maximum {
            return Err(CodecError {
                offset: length_offset,
                kind: CodecErrorKind::PolicyExceeded,
            });
        }
        self.fixed(length)
    }

    pub(crate) fn string(&mut self, maximum: usize) -> Result<String, CodecError> {
        let offset = self.position;
        let bytes = self.bytes(maximum)?;
        let text = std::str::from_utf8(bytes).map_err(|_| CodecError {
            offset,
            kind: CodecErrorKind::InvalidUtf8,
        })?;
        Ok(text.to_owned())
    }

    pub(crate) fn count(&mut self, maximum: usize) -> Result<usize, CodecError> {
        let offset = self.position;
        let value = self.u64()?;
        let count = usize::try_from(value).map_err(|_| CodecError {
            offset,
            kind: CodecErrorKind::LengthOverflow,
        })?;
        if count > maximum || count > self.remaining() {
            return Err(CodecError {
                offset,
                kind: CodecErrorKind::PolicyExceeded,
            });
        }
        Ok(count)
    }

    pub(crate) fn finish(self) -> Result<(), CodecError> {
        if self.position == self.bytes.len() {
            Ok(())
        } else {
            Err(CodecError {
                offset: self.position,
                kind: CodecErrorKind::TrailingBytes,
            })
        }
    }

    pub(crate) fn unknown_tag(&self, domain: TagDomain, tag: u8) -> CodecError {
        CodecError {
            offset: self.position.saturating_sub(1),
            kind: CodecErrorKind::UnknownTag { domain, tag },
        }
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], CodecError> {
        let bytes = self.fixed(N)?;
        let mut result = [0_u8; N];
        result.copy_from_slice(bytes);
        Ok(result)
    }

    fn error(&self, kind: CodecErrorKind) -> CodecError {
        CodecError {
            offset: self.position,
            kind,
        }
    }
}
