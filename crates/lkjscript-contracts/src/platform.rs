use crate::ContractDigest;

/// Sole lkjscript-owned monotonically increasing public number.
pub const PLATFORM_REVISION: u64 =
    parse_revision(include_bytes!("../../../meta/platform-revision"));

const fn parse_revision(bytes: &[u8]) -> u64 {
    if bytes.len() < 2 || bytes[bytes.len() - 1] != b'\n' || bytes[0] == b'0' {
        return 0;
    }
    let mut index = 0;
    let mut value = 0_u64;
    while index + 1 < bytes.len() {
        let byte = bytes[index];
        if byte < b'0' || byte > b'9' {
            return 0;
        }
        let Some(shifted) = value.checked_mul(10) else {
            return 0;
        };
        let Some(next) = shifted.checked_add((byte - b'0') as u64) else {
            return 0;
        };
        value = next;
        index += 1;
    }
    value
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicContractIdentity {
    pub platform_revision: u64,
    pub contract_digest: ContractDigest,
}

impl PublicContractIdentity {
    pub const fn new(contract_digest: ContractDigest) -> Self {
        Self {
            platform_revision: PLATFORM_REVISION,
            contract_digest,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_revision_file_is_canonical_and_nonzero() {
        assert_ne!(PLATFORM_REVISION, 0);
        assert_eq!(parse_revision(b"1\n"), 1);
        assert_eq!(parse_revision(b"18446744073709551615\n"), u64::MAX);
        for invalid in [
            b"0\n".as_slice(),
            b"01\n".as_slice(),
            b"1".as_slice(),
            b"1\r\n".as_slice(),
            b"18446744073709551616\n".as_slice(),
        ] {
            assert_eq!(parse_revision(invalid), 0);
        }
    }
}
