//! Canonical integrity envelope for packed semantic objects.

use super::diagnostic::{Diagnostic, DiagnosticClass};
pub const PACKED_ENVELOPE_VERSION: u16 = 1;
pub const MAXIMUM_PACKED_PAYLOAD_BYTES: usize = 128 * 1_048_576;
const HEADER_BYTES: usize = 8 + 2 + 8;
const CHECKSUM_BYTES: usize = 32;

pub fn encode<T: bincode::Encode>(
    magic: [u8; 8],
    digest_domain: &str,
    value: &T,
    maximum_payload_bytes: usize,
) -> Result<Vec<u8>, Diagnostic> {
    if maximum_payload_bytes > MAXIMUM_PACKED_PAYLOAD_BYTES {
        return Err(packed_error(
            DiagnosticClass::Infrastructure,
            "packed_limit_configuration",
            "packed object maximum exceeds the global decoder bound",
        ));
    }
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding()
        .with_limit::<MAXIMUM_PACKED_PAYLOAD_BYTES>();
    let payload = bincode::encode_to_vec(value, configuration).map_err(|error| {
        packed_error(
            DiagnosticClass::Infrastructure,
            "packed_encode",
            format!("packed object encoding failed: {error}"),
        )
    })?;
    if payload.len() > maximum_payload_bytes {
        return Err(packed_error(
            DiagnosticClass::Resource,
            "packed_payload_limit",
            format!(
                "packed object payload has {} bytes; the limit is {maximum_payload_bytes}",
                payload.len()
            ),
        ));
    }
    let payload_length = u64::try_from(payload.len()).map_err(|_| {
        packed_error(
            DiagnosticClass::Resource,
            "packed_payload_length",
            "packed object length cannot be represented",
        )
    })?;
    let capacity = HEADER_BYTES
        .checked_add(payload.len())
        .and_then(|value| value.checked_add(CHECKSUM_BYTES))
        .ok_or_else(|| {
            packed_error(
                DiagnosticClass::Resource,
                "packed_object_length",
                "packed object length overflowed",
            )
        })?;
    let mut bytes = Vec::with_capacity(capacity);
    bytes.extend_from_slice(&magic);
    bytes.extend_from_slice(&PACKED_ENVELOPE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&payload_length.to_le_bytes());
    bytes.extend_from_slice(&payload);
    let checksum = digest(digest_domain, &bytes);
    bytes.extend_from_slice(&checksum);
    Ok(bytes)
}

pub fn decode<T: bincode::Decode<()>>(
    bytes: &[u8],
    expected_magic: [u8; 8],
    digest_domain: &str,
    maximum_payload_bytes: usize,
) -> Result<T, Diagnostic> {
    if maximum_payload_bytes > MAXIMUM_PACKED_PAYLOAD_BYTES {
        return Err(packed_error(
            DiagnosticClass::Infrastructure,
            "packed_limit_configuration",
            "packed object maximum exceeds the global decoder bound",
        ));
    }
    let minimum = HEADER_BYTES + CHECKSUM_BYTES;
    if bytes.len() < minimum {
        return Err(packed_error(
            DiagnosticClass::Corrupt,
            "packed_truncated",
            "packed object is truncated",
        ));
    }
    if bytes[..8] != expected_magic {
        return Err(packed_error(
            DiagnosticClass::Source,
            "packed_contract",
            "packed object has an unknown contract identity",
        ));
    }
    let mut version_bytes = [0_u8; 2];
    version_bytes.copy_from_slice(&bytes[8..10]);
    let version = u16::from_le_bytes(version_bytes);
    if version != PACKED_ENVELOPE_VERSION {
        return Err(packed_error(
            DiagnosticClass::Source,
            "packed_envelope_version",
            format!(
                "packed envelope version {version} is not current version {PACKED_ENVELOPE_VERSION}"
            ),
        ));
    }
    let mut length_bytes = [0_u8; 8];
    length_bytes.copy_from_slice(&bytes[10..18]);
    let payload_length = usize::try_from(u64::from_le_bytes(length_bytes)).map_err(|_| {
        packed_error(
            DiagnosticClass::Resource,
            "packed_payload_length",
            "packed payload length cannot be represented",
        )
    })?;
    if payload_length > maximum_payload_bytes {
        return Err(packed_error(
            DiagnosticClass::Resource,
            "packed_payload_limit",
            format!("packed object payload exceeds {maximum_payload_bytes} bytes"),
        ));
    }
    let expected_length = HEADER_BYTES
        .checked_add(payload_length)
        .and_then(|value| value.checked_add(CHECKSUM_BYTES))
        .ok_or_else(|| {
            packed_error(
                DiagnosticClass::Resource,
                "packed_object_length",
                "packed object length overflowed",
            )
        })?;
    if bytes.len() != expected_length {
        return Err(packed_error(
            DiagnosticClass::Corrupt,
            "packed_length_mismatch",
            "packed object length does not match its canonical header",
        ));
    }
    let checksum_start = HEADER_BYTES + payload_length;
    let actual = digest(digest_domain, &bytes[..checksum_start]);
    if bytes[checksum_start..] != actual {
        return Err(packed_error(
            DiagnosticClass::Corrupt,
            "packed_checksum",
            "packed object checksum does not match its domain-separated content",
        ));
    }
    let payload = &bytes[HEADER_BYTES..checksum_start];
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding()
        .with_limit::<MAXIMUM_PACKED_PAYLOAD_BYTES>();
    let (value, consumed): (T, usize) = bincode::decode_from_slice(payload, configuration)
        .map_err(|error| {
            packed_error(
                DiagnosticClass::Corrupt,
                "packed_decode",
                format!("packed object payload is malformed: {error}"),
            )
        })?;
    if consumed != payload.len() {
        return Err(packed_error(
            DiagnosticClass::Corrupt,
            "packed_trailing",
            "packed object payload has trailing bytes",
        ));
    }
    Ok(value)
}

fn digest(domain: &str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn packed_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::{Decode, Encode};

    #[derive(Debug, Decode, Encode, Eq, PartialEq)]
    struct Fixture {
        name: String,
        values: Vec<u64>,
    }

    #[test]
    fn canonical_round_trip_and_integrity_failures() {
        let fixture = Fixture {
            name: "packed".to_owned(),
            values: vec![1, 2, 3],
        };
        let bytes =
            encode(*b"LKJTEST1", "lkjscript.test.packed.v1", &fixture, 1024).expect("encode");
        assert_eq!(
            decode::<Fixture>(&bytes, *b"LKJTEST1", "lkjscript.test.packed.v1", 1024)
                .expect("decode"),
            fixture
        );

        let mut corrupt = bytes.clone();
        corrupt[HEADER_BYTES] ^= 1;
        assert_eq!(
            decode::<Fixture>(&corrupt, *b"LKJTEST1", "lkjscript.test.packed.v1", 1024)
                .expect_err("checksum")
                .code,
            "packed_checksum"
        );

        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            decode::<Fixture>(&trailing, *b"LKJTEST1", "lkjscript.test.packed.v1", 1024)
                .expect_err("trailing")
                .code,
            "packed_length_mismatch"
        );
    }
}
