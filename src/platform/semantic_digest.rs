//! Domain-separated digests for semantic content and operational evidence.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

const DIGEST_BYTES: usize = 32;

macro_rules! semantic_digest {
    ($name:ident, $prefix:literal, $derive_key:literal, $tag:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; DIGEST_BYTES]);

        impl $name {
            pub const PREFIX: &'static str = $prefix;

            pub fn of(bytes: &[u8]) -> Self {
                let mut hasher = blake3::Hasher::new_derive_key($derive_key);
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

            pub fn parse(value: &str) -> Result<Self, Diagnostic> {
                value.parse()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(Self::PREFIX)?;
                formatter.write_str(&super::semantic_id::encode_hex(&self.0))
            }
        }

        impl FromStr for $name {
            type Err = Diagnostic;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let encoded = value.strip_prefix(Self::PREFIX).ok_or_else(|| {
                    digest_error(
                        "semantic_digest_domain",
                        format!(
                            "digest belongs to a foreign domain; expected '{}'",
                            Self::PREFIX
                        ),
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
                        "foreign semantic digest domain tag {tag}; expected {}",
                        $tag
                    )));
                }
                Ok(Self(<[u8; DIGEST_BYTES]>::decode(decoder)?))
            }
        }

        bincode::impl_borrow_decode!($name);
    };
}

semantic_digest!(
    ModuleObjectDigest,
    "module_object_",
    "lkjscript.module-object.v1",
    1u8
);
semantic_digest!(
    RootObjectDigest,
    "root_object_",
    "lkjscript.root-object.v1",
    2u8
);
semantic_digest!(
    SemanticDiffDigest,
    "diff_",
    "lkjscript.semantic-diff.v1",
    3u8
);
semantic_digest!(ReceiptDigest, "receipt_", "lkjscript.receipt.v1", 4u8);
semantic_digest!(
    TransactionDigest,
    "transaction_",
    "lkjscript.transaction.v1",
    5u8
);
semantic_digest!(IndexDigest, "index_", "lkjscript.index.v1", 6u8);
semantic_digest!(BackupDigest, "backup_", "lkjscript.backup.v1", 7u8);
semantic_digest!(ArtifactDigest, "artifact_", "lkjscript.artifact.v2", 8u8);
semantic_digest!(
    RevisionRecordDigest,
    "revision_record_",
    "lkjscript.revision-record.v1",
    9u8
);

fn decode_digest(encoded: &str) -> Result<[u8; DIGEST_BYTES], Diagnostic> {
    if encoded.len() != DIGEST_BYTES * 2 {
        return Err(digest_error(
            "semantic_digest_length",
            "semantic digest must contain exactly 64 lowercase hexadecimal characters",
        ));
    }
    let mut bytes = [0_u8; DIGEST_BYTES];
    for (index, pair) in encoded.as_bytes().chunks_exact(2).enumerate() {
        let high = decode_hex(pair[0]).ok_or_else(|| invalid_hex(encoded))?;
        let low = decode_hex(pair[1]).ok_or_else(|| invalid_hex(encoded))?;
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
        "semantic_digest_hex",
        format!("digest '{value}' is not canonical lowercase hexadecimal"),
    )
}

fn digest_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains_are_separate_in_text_and_packed_forms() {
        let module = ModuleObjectDigest::of(b"same");
        let root = RootObjectDigest::of(b"same");
        assert_ne!(module.bytes(), root.bytes());
        assert!(module.to_string().parse::<RootObjectDigest>().is_err());
        let configuration = bincode::config::standard();
        let encoded = bincode::encode_to_vec(module, configuration).expect("encode module");
        assert!(
            bincode::decode_from_slice::<RootObjectDigest, _>(&encoded, configuration).is_err()
        );
    }
}
