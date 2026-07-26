#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Utf8ErrorKind {
    UnexpectedContinuation,
    InvalidLeadingByte,
    MissingContinuation,
    OverlongEncoding,
    Surrogate,
    OutOfRange,
}

impl Utf8ErrorKind {
    pub(crate) const fn variant_id(self) -> [u8; 32] {
        match self {
            Self::UnexpectedContinuation => [
                0x33, 0x4a, 0xb5, 0x99, 0x7e, 0xee, 0x38, 0x6d, 0x4f, 0x2b, 0x0d, 0x75, 0xd0, 0xe0,
                0x1d, 0x2c, 0xe7, 0xcf, 0x08, 0x01, 0x46, 0x8f, 0xf3, 0x7a, 0xf0, 0xec, 0x64, 0xe1,
                0x99, 0x67, 0x33, 0xd3,
            ],
            Self::InvalidLeadingByte => [
                0x29, 0x0e, 0x4a, 0x15, 0xe5, 0x1b, 0x3e, 0x7f, 0x0e, 0xf2, 0x52, 0x62, 0xcb, 0xdf,
                0x4a, 0xc8, 0x9b, 0x0c, 0x80, 0x09, 0x58, 0x26, 0xac, 0x19, 0x8c, 0x36, 0x75, 0x72,
                0x25, 0x13, 0x6c, 0x6d,
            ],
            Self::MissingContinuation => [
                0xd2, 0xb5, 0x7f, 0x67, 0x23, 0x23, 0xc8, 0x5f, 0x7f, 0x9b, 0x29, 0xed, 0x3b, 0x83,
                0x8f, 0xe3, 0xba, 0xd0, 0x73, 0xa0, 0xc6, 0x10, 0xde, 0xc4, 0xf9, 0x36, 0xcd, 0x88,
                0xbf, 0x33, 0xdd, 0x3d,
            ],
            Self::OverlongEncoding => [
                0xf3, 0x50, 0xd7, 0x17, 0x15, 0xa1, 0x2e, 0xcd, 0x78, 0x21, 0xa4, 0x0b, 0xb6, 0x16,
                0x9e, 0x16, 0x56, 0x08, 0xac, 0xfd, 0x53, 0x6c, 0x08, 0x9a, 0xd8, 0xf5, 0x54, 0x6f,
                0x98, 0x54, 0x97, 0x26,
            ],
            Self::Surrogate => [
                0x57, 0xef, 0xa2, 0x9c, 0x38, 0x30, 0xf9, 0xf2, 0xbf, 0xe8, 0x3e, 0x7b, 0xf1, 0xd2,
                0x82, 0x87, 0xc1, 0xd9, 0x7f, 0x1c, 0x0a, 0xf9, 0x65, 0xc8, 0x06, 0x4f, 0x9a, 0x9e,
                0xb6, 0x66, 0xe3, 0xac,
            ],
            Self::OutOfRange => [
                0xba, 0x0b, 0x1d, 0xb1, 0x65, 0x22, 0x74, 0x7c, 0x48, 0x69, 0x82, 0x2d, 0x9f, 0x07,
                0xe3, 0xb0, 0x26, 0xc5, 0x83, 0x18, 0x3c, 0xc6, 0x96, 0xc9, 0x92, 0xe2, 0x17, 0xd0,
                0xdc, 0x5a, 0x0c, 0x78,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Utf8Failure {
    pub(crate) offset: usize,
    pub(crate) kind: Utf8ErrorKind,
}

pub(crate) fn validate_utf8(bytes: &[u8]) -> Result<&str, Utf8Failure> {
    let mut offset = 0;
    while offset < bytes.len() {
        let first = bytes[offset];
        if first < 0x80 {
            offset += 1;
            continue;
        }
        let width = match first {
            0x80..=0xbf => return failure(offset, Utf8ErrorKind::UnexpectedContinuation),
            0xc0..=0xc1 => return failure(offset, Utf8ErrorKind::OverlongEncoding),
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            0xf5..=0xf7 => return failure(offset, Utf8ErrorKind::OutOfRange),
            _ => return failure(offset, Utf8ErrorKind::InvalidLeadingByte),
        };
        if bytes.len().saturating_sub(offset) < width {
            return failure(offset, Utf8ErrorKind::MissingContinuation);
        }
        let second = bytes[offset + 1];
        if !(0x80..=0xbf).contains(&second) {
            return failure(offset, Utf8ErrorKind::MissingContinuation);
        }
        match (first, second) {
            (0xe0, 0x80..=0x9f) | (0xf0, 0x80..=0x8f) => {
                return failure(offset, Utf8ErrorKind::OverlongEncoding)
            }
            (0xed, 0xa0..=0xbf) => return failure(offset, Utf8ErrorKind::Surrogate),
            (0xf4, 0x90..=0xbf) => return failure(offset, Utf8ErrorKind::OutOfRange),
            _ => {}
        }
        for continuation in &bytes[offset + 2..offset + width] {
            if !(0x80..=0xbf).contains(continuation) {
                return failure(offset, Utf8ErrorKind::MissingContinuation);
            }
        }
        offset += width;
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(_) => failure(0, Utf8ErrorKind::InvalidLeadingByte),
    }
}

fn failure<T>(offset: usize, kind: Utf8ErrorKind) -> Result<T, Utf8Failure> {
    Err(Utf8Failure { offset, kind })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_free_classifier_covers_the_closed_error_set() {
        let cases = [
            (&[0x80][..], Utf8ErrorKind::UnexpectedContinuation),
            (&[0xff][..], Utf8ErrorKind::InvalidLeadingByte),
            (&[0xe2, b'A'][..], Utf8ErrorKind::MissingContinuation),
            (&[0xc0, 0x80][..], Utf8ErrorKind::OverlongEncoding),
            (&[0xed, 0xa0, 0x80][..], Utf8ErrorKind::Surrogate),
            (&[0xf4, 0x90, 0x80, 0x80][..], Utf8ErrorKind::OutOfRange),
        ];
        for (bytes, kind) in cases {
            assert_eq!(validate_utf8(bytes), Err(Utf8Failure { offset: 0, kind }));
        }
        assert_eq!(validate_utf8(b"ok"), Ok("ok"));
    }
}
