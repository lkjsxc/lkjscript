//! Canonical integrity envelope for packed semantic objects.

use super::canonical_schema::CANONICAL_SCHEMA_HASH;
use super::diagnostic::{Diagnostic, DiagnosticClass};
pub const MAXIMUM_PACKED_PAYLOAD_BYTES: usize = 128 * 1_048_576;
pub(crate) const CANONICAL_HEADER_BYTES: usize = 8 + 32 + 8;
const HEADER_BYTES: usize = CANONICAL_HEADER_BYTES;
const CHECKSUM_BYTES: usize = 32;
pub(crate) const CANONICAL_ENVELOPE_BYTES: usize = HEADER_BYTES + CHECKSUM_BYTES;

pub(crate) use decode as decode_canonical;
pub(crate) use encode as encode_canonical;

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
    bytes.extend_from_slice(&CANONICAL_SCHEMA_HASH);
    bytes.extend_from_slice(&payload_length.to_le_bytes());
    bytes.extend_from_slice(&payload);
    let checksum = digest(digest_domain, &bytes);
    bytes.extend_from_slice(&checksum);
    Ok(bytes)
}

pub fn decode<T: bincode::Decode<()> + bincode::Encode>(
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
    if bytes[8..40] != CANONICAL_SCHEMA_HASH {
        return Err(packed_error(
            DiagnosticClass::Source,
            "packed_schema",
            "packed object does not use the current canonical schema",
        ));
    }
    let mut length_bytes = [0_u8; 8];
    length_bytes.copy_from_slice(&bytes[40..48]);
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
    let canonical = bincode::encode_to_vec(&value, configuration).map_err(|error| {
        packed_error(
            DiagnosticClass::Corrupt,
            "packed_decode",
            format!("packed object cannot reproduce its canonical payload: {error}"),
        )
    })?;
    if canonical != payload {
        return Err(packed_error(
            DiagnosticClass::Corrupt,
            "packed_noncanonical",
            "packed object does not use its canonical binary encoding",
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
    fn valid_checksum_does_not_admit_nonminimal_binary_integers() {
        let canonical =
            encode(*b"LKJTEST1", "lkjscript.test.packed.v1", &1_u16, 1024).expect("encode integer");
        let mut nonminimal = canonical[..HEADER_BYTES].to_vec();
        nonminimal[40..48].copy_from_slice(&3_u64.to_le_bytes());
        nonminimal.extend_from_slice(&[251, 1, 0]);
        let checksum = digest("lkjscript.test.packed.v1", &nonminimal);
        nonminimal.extend_from_slice(&checksum);
        assert_eq!(
            decode::<u16>(&nonminimal, *b"LKJTEST1", "lkjscript.test.packed.v1", 1024)
                .expect_err("nonminimal integer with valid checksum")
                .code,
            "packed_noncanonical"
        );
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

        let mut foreign_schema = bytes.clone();
        foreign_schema[8] ^= 1;
        let checksum_start = foreign_schema.len() - CHECKSUM_BYTES;
        let checksum = digest(
            "lkjscript.test.packed.v1",
            &foreign_schema[..checksum_start],
        );
        foreign_schema[checksum_start..].copy_from_slice(&checksum);
        assert_eq!(
            decode::<Fixture>(
                &foreign_schema,
                *b"LKJTEST1",
                "lkjscript.test.packed.v1",
                1024
            )
            .expect_err("foreign schema with valid checksum")
            .code,
            "packed_schema"
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
