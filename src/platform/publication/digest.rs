//! Typed domain-separated Graph 6 accepted-history object digests.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

const DIGEST_BYTES: usize = 32;

macro_rules! history_digest {
    ($name:ident, $prefix:literal, $domain:expr, $tag:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; DIGEST_BYTES]);

        impl $name {
            pub const PREFIX: &'static str = $prefix;
            pub const DOMAIN: &'static str = $domain;

            pub fn of(bytes: &[u8]) -> Self {
                let mut hasher = blake3::Hasher::new_derive_key(Self::DOMAIN);
                hasher.update(&(bytes.len() as u64).to_be_bytes());
                hasher.update(bytes);
                Self(*hasher.finalize().as_bytes())
            }

            pub const fn from_bytes(bytes: [u8; DIGEST_BYTES]) -> Self {
                Self(bytes)
            }

            pub const fn bytes(self) -> [u8; DIGEST_BYTES] {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(Self::PREFIX)?;
                formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
            }
        }

        impl FromStr for $name {
            type Err = Diagnostic;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let encoded = value.strip_prefix(Self::PREFIX).ok_or_else(|| {
                    digest_error(
                        "publication_digest_domain",
                        format!("history digest must start with '{}'", Self::PREFIX),
                    )
                })?;
                decode_digest(encoded).map(Self)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                value.parse().map_err(serde::de::Error::custom)
            }
        }

        impl Encode for $name {
            fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
                $tag.encode(encoder)?;
                self.0.encode(encoder)
            }
        }

        impl<Context> Decode<Context> for $name {
            fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
                let tag = u8::decode(decoder)?;
                if tag != $tag {
                    return Err(DecodeError::OtherString(format!(
                        "foreign history digest domain tag {tag}; expected {}",
                        $tag
                    )));
                }
                Ok(Self(<[u8; DIGEST_BYTES]>::decode(decoder)?))
            }
        }

        bincode::impl_borrow_decode!($name);
    };
}

history_digest!(
    RevisionObjectDigest,
    "revision_object_",
    crate::platform::storage::contract::REVISION_OBJECT_DIGEST_DOMAIN,
    1u8
);
history_digest!(
    ReceiptObjectDigest,
    "receipt_object_",
    crate::platform::storage::contract::RECEIPT_OBJECT_DIGEST_DOMAIN,
    2u8
);
history_digest!(
    TransactionDigest,
    "transaction_",
    crate::platform::storage::contract::TRANSACTION_OBJECT_DIGEST_DOMAIN,
    3u8
);
history_digest!(
    SemanticDiffDigest,
    "semantic_diff_",
    crate::platform::storage::contract::SEMANTIC_DIFF_OBJECT_DIGEST_DOMAIN,
    4u8
);

fn decode_digest(value: &str) -> Result<[u8; DIGEST_BYTES], Diagnostic> {
    if value.len() != DIGEST_BYTES * 2 {
        return Err(digest_error(
            "publication_digest_length",
            "history digest must contain 64 lowercase hexadecimal characters",
        ));
    }
    let mut bytes = [0_u8; DIGEST_BYTES];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let high = decode_hex(pair[0]).ok_or_else(|| invalid_hex(value))?;
        let low = decode_hex(pair[1]).ok_or_else(|| invalid_hex(value))?;
        bytes[index] = (high << 4) | low;
    }
    Ok(bytes)
}

fn decode_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn invalid_hex(value: &str) -> Diagnostic {
    digest_error(
        "publication_digest_hex",
        format!("history digest '{value}' is not canonical lowercase hexadecimal"),
    )
}

fn digest_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}
