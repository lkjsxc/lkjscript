//! Typed witness, summary, and semantic-dimension digests.

use super::contract;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

const DIGEST_BYTES: usize = 32;

macro_rules! witness_digest {
    ($name:ident, $prefix:literal, $domain:expr, $tag:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; DIGEST_BYTES]);

        impl $name {
            pub const PREFIX: &'static str = $prefix;
            pub const DOMAIN: &'static str = $domain;

            pub fn of(bytes: &[u8]) -> Self {
                Self(domain_digest(Self::DOMAIN, bytes))
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
                    digest_error(format!("digest must start with '{}'", Self::PREFIX))
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

        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> {
                concat!("lkjscript.", stringify!($name), "V5").into()
            }

            fn schema_id() -> Cow<'static, str> {
                concat!(module_path!(), "::", stringify!($name)).into()
            }

            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                let pattern = format!("^{}[0-9a-f]{{64}}$", Self::PREFIX);
                json_schema!({
                    "type": "string",
                    "minLength": Self::PREFIX.len() + 64,
                    "maxLength": Self::PREFIX.len() + 64,
                    "pattern": pattern
                })
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
                        "foreign witness digest domain tag {tag}; expected {}",
                        $tag
                    )));
                }
                Ok(Self(<[u8; DIGEST_BYTES]>::decode(decoder)?))
            }
        }

        bincode::impl_borrow_decode!($name);
    };
}

witness_digest!(
    OwnerSummaryDigest,
    "owner_summary_",
    contract::OWNER_SUMMARY_DIGEST_DOMAIN,
    1u8
);
witness_digest!(
    ValidationWitnessDigest,
    "validation_witness_",
    contract::VALIDATION_WITNESS_DIGEST_DOMAIN,
    2u8
);
witness_digest!(
    ValidationCertificateDigest,
    "validation_certificate_",
    contract::VALIDATION_CERTIFICATE_DIGEST_DOMAIN,
    3u8
);
witness_digest!(
    ValidatorContractDigest,
    "validator_contract_",
    contract::VALIDATOR_CONTRACT_DIGEST_DOMAIN,
    4u8
);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticDigest([u8; DIGEST_BYTES]);

impl SemanticDigest {
    pub fn of(domain: &str, bytes: &[u8]) -> Self {
        Self(domain_digest(domain, bytes))
    }

    pub const fn from_bytes(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; DIGEST_BYTES] {
        self.0
    }
}

impl Encode for SemanticDigest {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        5_u8.encode(encoder)?;
        self.0.encode(encoder)
    }
}

impl<Context> Decode<Context> for SemanticDigest {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let tag = u8::decode(decoder)?;
        if tag != 5 {
            return Err(DecodeError::OtherString(format!(
                "foreign semantic-dimension digest tag {tag}"
            )));
        }
        Ok(Self(<[u8; DIGEST_BYTES]>::decode(decoder)?))
    }
}

bincode::impl_borrow_decode!(SemanticDigest);

fn domain_digest(domain: &str, bytes: &[u8]) -> [u8; DIGEST_BYTES] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn decode_digest(value: &str) -> Result<[u8; DIGEST_BYTES], Diagnostic> {
    if value.len() != DIGEST_BYTES * 2 {
        return Err(digest_error(
            "witness digest must contain 64 lowercase hexadecimal characters",
        ));
    }
    let mut bytes = [0_u8; DIGEST_BYTES];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let high = decode_hex(pair[0]).ok_or_else(|| digest_error("noncanonical digest hex"))?;
        let low = decode_hex(pair[1]).ok_or_else(|| digest_error("noncanonical digest hex"))?;
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

fn digest_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, "witness_digest", message)
}
