//! Optional exact payload policy for audited, extent-driven history decoders.
//!
//! Generic bincode collection decoders reserve the entire declared count before
//! reading its elements. These entry points require progressive fallible sequence
//! decoding instead, so removing a quota cannot make a tiny malformed payload
//! request an arbitrary allocation. Other recursive codecs retain their existing
//! admission until their own complete lifecycle is converted.

use super::{
    CANONICAL_ENVELOPE_BYTES, CanonicalPayload, HEADER_BYTES, admit_payload, digest, packed_error,
};
use crate::platform::canonical_schema::SCHEMA_EPOCH;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::{Decode, Encode};

/// Implement only after every variable collection in the decoded type uses
/// extent-driven, fallible allocation, including collections in nested records.
pub(crate) trait LinearDecode: Decode<()> {}

const ALLOCATION_FAILURE: &str = "packed sequence allocation failed";

/// Each concrete history element has a nonempty encoding. Decode it before
/// reserving its slot: malformed counts can advance only over available input.
pub(crate) fn decode_vec<
    Context,
    T: Decode<Context>,
    D: bincode::de::Decoder<Context = Context>,
>(
    decoder: &mut D,
) -> Result<Vec<T>, bincode::error::DecodeError> {
    let length = usize::decode(decoder)?;
    let mut values = Vec::new();
    for _ in 0..length {
        let value = T::decode(decoder)?;
        values
            .try_reserve(1)
            .map_err(|_| bincode::error::DecodeError::Other(ALLOCATION_FAILURE))?;
        values.push(value);
    }
    Ok(values)
}

#[derive(Clone, Copy)]
enum WriteFailure {
    Limit,
    Length,
    Allocation,
}

struct PayloadWriter {
    bytes: Vec<u8>,
    maximum: Option<usize>,
    failure: Option<WriteFailure>,
}

impl bincode::enc::write::Writer for PayloadWriter {
    fn write(&mut self, bytes: &[u8]) -> Result<(), bincode::error::EncodeError> {
        let current = self.bytes.len() - HEADER_BYTES;
        let Some(required) = current.checked_add(bytes.len()) else {
            self.failure = Some(WriteFailure::Length);
            return Err(bincode::error::EncodeError::Other(
                "packed payload length overflow",
            ));
        };
        if self.maximum.is_some_and(|maximum| required > maximum) {
            self.failure = Some(WriteFailure::Limit);
            return Err(bincode::error::EncodeError::Other(
                "packed payload allowance exhausted",
            ));
        }
        if self.bytes.try_reserve(bytes.len()).is_err() {
            self.failure = Some(WriteFailure::Allocation);
            return Err(bincode::error::EncodeError::Other(ALLOCATION_FAILURE));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

pub(crate) fn encode<T: Encode>(
    magic: [u8; 8],
    domain: &str,
    value: &T,
    maximum: Option<usize>,
) -> Result<Vec<u8>, Diagnostic> {
    let mut bytes = Vec::new();
    bytes.try_reserve(CANONICAL_ENVELOPE_BYTES).map_err(|_| {
        packed_error(
            DiagnosticClass::Resource,
            "packed_allocation",
            "cannot allocate packed envelope",
        )
    })?;
    bytes.extend_from_slice(&magic);
    bytes.extend_from_slice(&SCHEMA_EPOCH.to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    let mut writer = PayloadWriter {
        bytes,
        maximum,
        failure: None,
    };
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding()
        .with_no_limit();
    if let Err(error) = bincode::encode_into_writer(value, &mut writer, configuration) {
        let (class, code) = match writer.failure {
            Some(WriteFailure::Limit) => (DiagnosticClass::Resource, "packed_payload_limit"),
            Some(WriteFailure::Length) => (DiagnosticClass::Resource, "packed_payload_length"),
            Some(WriteFailure::Allocation) => (DiagnosticClass::Resource, "packed_allocation"),
            None => (DiagnosticClass::Infrastructure, "packed_encode"),
        };
        return Err(packed_error(
            class,
            code,
            format!("packed object encoding failed: {error}"),
        ));
    }
    let length = u64::try_from(writer.bytes.len() - HEADER_BYTES).map_err(|_| {
        packed_error(
            DiagnosticClass::Resource,
            "packed_payload_length",
            "packed payload length is not representable",
        )
    })?;
    writer.bytes[10..18].copy_from_slice(&length.to_le_bytes());
    let checksum = digest(domain, &writer.bytes);
    writer.bytes.try_reserve(checksum.len()).map_err(|_| {
        packed_error(
            DiagnosticClass::Resource,
            "packed_allocation",
            "cannot allocate packed checksum",
        )
    })?;
    writer.bytes.extend_from_slice(&checksum);
    Ok(writer.bytes)
}

fn decode<T: LinearDecode>(
    bytes: &[u8],
    magic: [u8; 8],
    domain: &str,
    maximum: Option<usize>,
) -> Result<T, Diagnostic> {
    let payload = admit_payload(bytes, magic, domain, maximum)?;
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding()
        .with_no_limit();
    let (value, consumed): (T, usize) = bincode::decode_from_slice(payload, configuration)
        .map_err(|error| {
            let (class, code) = if matches!(
                error,
                bincode::error::DecodeError::Other(ALLOCATION_FAILURE)
            ) {
                (DiagnosticClass::Resource, "packed_allocation")
            } else {
                (DiagnosticClass::Corrupt, "packed_decode")
            };
            packed_error(
                class,
                code,
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

pub(crate) fn decode_canonical<T: LinearDecode + Encode>(
    bytes: &[u8],
    magic: [u8; 8],
    domain: &str,
    maximum: Option<usize>,
) -> Result<T, Diagnostic> {
    let value = decode(bytes, magic, domain, maximum)?;
    let payload = &bytes[HEADER_BYTES..bytes.len() - 32];
    let mut canonical = CanonicalPayload {
        remaining: payload,
        mismatch: false,
    };
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding()
        .with_no_limit();
    let encoded = bincode::encode_into_writer(&value, &mut canonical, configuration);
    if canonical.mismatch || encoded.is_ok() && !canonical.remaining.is_empty() {
        return Err(packed_error(
            DiagnosticClass::Corrupt,
            "packed_noncanonical",
            "packed object does not use its canonical binary encoding",
        ));
    }
    encoded.map_err(|error| {
        packed_error(
            DiagnosticClass::Corrupt,
            "packed_decode",
            format!("packed object cannot reproduce its canonical payload: {error}"),
        )
    })?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    const MAGIC: [u8; 8] = *b"LKJLIN01";
    const DOMAIN: &str = "lkjscript.test.linear-envelope";

    #[derive(Debug, Encode, PartialEq)]
    struct Values(Vec<u64>);
    impl<Context> Decode<Context> for Values {
        fn decode<D: bincode::de::Decoder<Context = Context>>(
            decoder: &mut D,
        ) -> Result<Self, bincode::error::DecodeError> {
            Ok(Self(decode_vec(decoder)?))
        }
    }
    impl LinearDecode for Values {}

    #[test]
    fn optional_payload_policy_is_exact_and_never_an_unbounded_sentinel() {
        let value = Values(vec![0, 251, u64::MAX]);
        let bytes = encode(MAGIC, DOMAIN, &value, None).unwrap();
        let exact = bytes.len() - CANONICAL_ENVELOPE_BYTES;
        for maximum in [None, Some(exact), Some(usize::MAX)] {
            assert_eq!(encode(MAGIC, DOMAIN, &value, maximum).unwrap(), bytes);
            assert_eq!(
                decode_canonical::<Values>(&bytes, MAGIC, DOMAIN, maximum).unwrap(),
                value
            );
        }
        for maximum in [0, exact - 1] {
            assert_eq!(
                encode(MAGIC, DOMAIN, &value, Some(maximum))
                    .unwrap_err()
                    .code,
                "packed_payload_limit"
            );
            assert_eq!(
                decode_canonical::<Values>(&bytes, MAGIC, DOMAIN, Some(maximum))
                    .unwrap_err()
                    .code,
                "packed_payload_limit"
            );
        }
    }

    #[test]
    fn checksum_consistent_impossible_sequence_does_not_preallocate_its_count() {
        let bytes = encode(MAGIC, DOMAIN, &u64::MAX, None).unwrap();
        assert_eq!(
            decode_canonical::<Values>(&bytes, MAGIC, DOMAIN, None)
                .unwrap_err()
                .code,
            "packed_decode"
        );
        let mut nonminimal = bytes[..HEADER_BYTES].to_vec();
        nonminimal[10..18].copy_from_slice(&4_u64.to_le_bytes());
        nonminimal.extend_from_slice(&[1, 251, 1, 0]);
        nonminimal.extend_from_slice(&digest(DOMAIN, &nonminimal));
        assert_eq!(
            decode_canonical::<Values>(&nonminimal, MAGIC, DOMAIN, None)
                .unwrap_err()
                .code,
            "packed_noncanonical"
        );
    }
}
