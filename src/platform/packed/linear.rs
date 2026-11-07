//! Optional exact payload policy for audited, extent-driven decoders.
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

/// Each element must have a nonempty encoding. Decode it before reserving its
/// slot: malformed counts can advance only over available input. The caller of
/// the length-explicit form has already decoded and admitted its wire count.
pub(crate) fn decode_vec_len_with<
    Context,
    T,
    D: bincode::de::Decoder<Context = Context>,
    F: FnMut(&mut D) -> Result<T, bincode::error::DecodeError>,
>(
    decoder: &mut D,
    length: usize,
    mut read_element: F,
) -> Result<Vec<T>, bincode::error::DecodeError> {
    let mut values = Vec::new();
    for _ in 0..length {
        let value = read_element(decoder)?;
        values
            .try_reserve(1)
            .map_err(|_| bincode::error::DecodeError::Other(ALLOCATION_FAILURE))?;
        values.push(value);
    }
    Ok(values)
}

pub(crate) fn decode_vec_with<
    Context,
    T,
    D: bincode::de::Decoder<Context = Context>,
    F: FnMut(&mut D) -> Result<T, bincode::error::DecodeError>,
>(
    decoder: &mut D,
    read_element: F,
) -> Result<Vec<T>, bincode::error::DecodeError> {
    let length = usize::decode(decoder)?;
    decode_vec_len_with(decoder, length, read_element)
}

pub(crate) fn decode_vec<
    Context,
    T: Decode<Context>,
    D: bincode::de::Decoder<Context = Context>,
>(
    decoder: &mut D,
) -> Result<Vec<T>, bincode::error::DecodeError> {
    decode_vec_with(decoder, T::decode)
}

/// Slice readers prove the complete byte extent before reserving it. Readers
/// without a contiguous view fall back to progressive element decoding.
pub(crate) fn decode_bytes<Context, D: bincode::de::Decoder<Context = Context>>(
    decoder: &mut D,
) -> Result<Vec<u8>, bincode::error::DecodeError> {
    use bincode::de::read::Reader;
    let length = usize::decode(decoder)?;
    decoder.claim_bytes_read(length)?;
    if let Some(source) = decoder
        .reader()
        .peek_read(length)
        .and_then(|source| source.get(..length))
    {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(source.len())
            .map_err(|_| bincode::error::DecodeError::Other(ALLOCATION_FAILURE))?;
        bytes.extend_from_slice(source);
        decoder.reader().consume(length);
        return Ok(bytes);
    }
    decoder.unclaim_bytes_read(length);
    decode_vec_len_with(decoder, length, u8::decode)
}

pub(crate) fn decode_string<Context, D: bincode::de::Decoder<Context = Context>>(
    decoder: &mut D,
) -> Result<String, bincode::error::DecodeError> {
    String::from_utf8(decode_bytes(decoder)?).map_err(|error| bincode::error::DecodeError::Utf8 {
        inner: error.utf8_error(),
    })
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
    #[derive(Debug, Encode, PartialEq)]
    struct Strings(Vec<String>);
    impl<Context> Decode<Context> for Strings {
        fn decode<D: bincode::de::Decoder<Context = Context>>(
            decoder: &mut D,
        ) -> Result<Self, bincode::error::DecodeError> {
            Ok(Self(decode_vec_with(decoder, decode_string)?))
        }
    }
    impl LinearDecode for Strings {}

    #[test]
    fn progressive_nested_strings_preserve_bytes_and_reject_impossible_extents() {
        let value = Strings(vec![String::new(), "a".into(), "λ🙂".repeat(4097)]);
        let bytes = encode(MAGIC, DOMAIN, &value, None).unwrap();
        let standard =
            bincode::encode_to_vec(&value, bincode::config::standard().with_no_limit()).unwrap();
        assert_eq!(&bytes[HEADER_BYTES..bytes.len() - 32], standard);
        assert_eq!(
            decode_canonical::<Strings>(&bytes, MAGIC, DOMAIN, None).unwrap(),
            value
        );

        // Each tuple emits a vector length followed by a string length without
        // its bytes. A valid envelope must not authorize that declared reserve.
        for declared in [1_u64, u64::MAX] {
            let malformed = encode(MAGIC, DOMAIN, &(1_u8, declared), None).unwrap();
            assert_eq!(
                decode_canonical::<Strings>(&malformed, MAGIC, DOMAIN, None)
                    .unwrap_err()
                    .code,
                "packed_decode"
            );
        }
        let invalid_utf8 = encode(MAGIC, DOMAIN, &(1_u8, 1_u8, 0xff_u8), None).unwrap();
        assert_eq!(
            decode_canonical::<Strings>(&invalid_utf8, MAGIC, DOMAIN, None)
                .unwrap_err()
                .code,
            "packed_decode"
        );
    }

    #[test]
    fn progressive_string_decode_supports_non_peeking_readers_and_exact_limits() {
        #[derive(Debug, PartialEq)]
        struct Text(String);
        impl<Context> Decode<Context> for Text {
            fn decode<D: bincode::de::Decoder<Context = Context>>(
                decoder: &mut D,
            ) -> Result<Self, bincode::error::DecodeError> {
                Ok(Self(decode_string(decoder)?))
            }
        }

        let configuration = bincode::config::standard().with_no_limit();
        let bytes = bincode::encode_to_vec("λ🙂", configuration).unwrap();
        let mut stream = std::io::Cursor::new(&bytes);
        assert_eq!(
            bincode::decode_from_std_read::<Text, _, _>(&mut stream, configuration).unwrap(),
            Text("λ🙂".into())
        );
        let truncated = &bytes[..bytes.len() - 1];
        let mut stream = std::io::Cursor::new(truncated);
        assert!(bincode::decode_from_std_read::<Text, _, _>(&mut stream, configuration).is_err());

        let ascii = bincode::encode_to_vec("abc", configuration).unwrap();
        // Bincode charges the usize length field's in-memory width, then bytes.
        let limited = bincode::config::standard().with_limit::<11>();
        assert_eq!(
            bincode::decode_from_slice::<Text, _>(&ascii, limited)
                .unwrap()
                .0,
            Text("abc".into())
        );
        assert!(
            bincode::decode_from_slice::<Text, _>(
                &ascii,
                bincode::config::standard().with_limit::<10>(),
            )
            .is_err()
        );
    }
}
