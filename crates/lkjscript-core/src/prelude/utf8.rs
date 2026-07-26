use super::{Utf8ErrorKind as Kind, Utf8Failure};

pub fn validate_utf8(bytes: &[u8]) -> Result<&str, Utf8Failure> {
    let mut offset = 0;
    while offset < bytes.len() {
        let first = bytes[offset];
        if first < 0x80 {
            offset += 1;
            continue;
        }
        let width = match first {
            0x80..=0xbf => return failure(offset, Kind::UnexpectedContinuation),
            0xc0..=0xc1 => return failure(offset, Kind::OverlongEncoding),
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            0xf5..=0xf7 => return failure(offset, Kind::OutOfRange),
            _ => return failure(offset, Kind::InvalidLeadingByte),
        };
        if bytes.len().saturating_sub(offset) < width {
            return failure(offset, Kind::MissingContinuation);
        }
        let second = bytes[offset + 1];
        if !(0x80..=0xbf).contains(&second) {
            return failure(offset, Kind::MissingContinuation);
        }
        match (first, second) {
            (0xe0, 0x80..=0x9f) | (0xf0, 0x80..=0x8f) => {
                return failure(offset, Kind::OverlongEncoding)
            }
            (0xed, 0xa0..=0xbf) => return failure(offset, Kind::Surrogate),
            (0xf4, 0x90..=0xbf) => return failure(offset, Kind::OutOfRange),
            _ => {}
        }
        for continuation in &bytes[offset + 2..offset + width] {
            if !(0x80..=0xbf).contains(continuation) {
                return failure(offset, Kind::MissingContinuation);
            }
        }
        offset += width;
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(_) => failure(0, Kind::InvalidLeadingByte),
    }
}

fn failure<T>(offset: usize, kind: Kind) -> Result<T, Utf8Failure> {
    Err(Utf8Failure { offset, kind })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_every_closed_utf8_error_kind() {
        let cases = [
            (&[0x80][..], Kind::UnexpectedContinuation),
            (&[0xff][..], Kind::InvalidLeadingByte),
            (&[0xe2, b'A'][..], Kind::MissingContinuation),
            (&[0xc0, 0x80][..], Kind::OverlongEncoding),
            (&[0xed, 0xa0, 0x80][..], Kind::Surrogate),
            (&[0xf4, 0x90, 0x80, 0x80][..], Kind::OutOfRange),
        ];
        for (bytes, expected) in cases {
            assert_eq!(
                validate_utf8(bytes),
                Err(Utf8Failure {
                    offset: 0,
                    kind: expected
                })
            );
        }
        assert_eq!(validate_utf8(b"ok"), Ok("ok"));
    }
}
