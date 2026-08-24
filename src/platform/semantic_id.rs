//! Domain-separated stable semantic identities.
//!
//! Identity bytes are opaque continuity tokens. Human names, content digests, revision digests,
//! physical object keys, dense compiler indexes, and runtime handles are deliberately separate.

use super::contract::registry::{
    IDENTITY_MIGRATION_DIGEST_DOMAIN, REQUEST_LOCAL_IDENTITY_DIGEST_DOMAIN,
};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

const IDENTITY_BYTES: usize = 16;
const ENCODED_IDENTITY_BYTES: usize = IDENTITY_BYTES * 2;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct IdentityBytes([u8; IDENTITY_BYTES]);

impl IdentityBytes {
    fn random() -> Result<Self, Diagnostic> {
        let mut bytes = [0_u8; IDENTITY_BYTES];
        getrandom::fill(&mut bytes).map_err(|_| {
            identity_error(
                DiagnosticClass::Infrastructure,
                "semantic_identity_entropy",
                "operating-system entropy is unavailable",
            )
        })?;
        if bytes == [0; IDENTITY_BYTES] {
            bytes[IDENTITY_BYTES - 1] = 1;
        }
        Ok(Self(bytes))
    }

    fn deterministic(domain: &str, seed: &[u8], ordinal: u64) -> Self {
        let mut hasher = blake3::Hasher::new_derive_key(IDENTITY_MIGRATION_DIGEST_DOMAIN);
        hasher.update(&(domain.len() as u64).to_be_bytes());
        hasher.update(domain.as_bytes());
        hasher.update(&(seed.len() as u64).to_be_bytes());
        hasher.update(seed);
        hasher.update(&ordinal.to_be_bytes());
        let mut bytes = [0_u8; IDENTITY_BYTES];
        bytes.copy_from_slice(&hasher.finalize().as_bytes()[..IDENTITY_BYTES]);
        if bytes == [0; IDENTITY_BYTES] {
            bytes[IDENTITY_BYTES - 1] = 1;
        }
        Self(bytes)
    }

    fn request_local(domain: &str, seed: &[u8], ordinal: u64) -> Self {
        let mut hasher = blake3::Hasher::new_derive_key(REQUEST_LOCAL_IDENTITY_DIGEST_DOMAIN);
        hasher.update(&(domain.len() as u64).to_be_bytes());
        hasher.update(domain.as_bytes());
        hasher.update(&(seed.len() as u64).to_be_bytes());
        hasher.update(seed);
        hasher.update(&ordinal.to_be_bytes());
        let mut bytes = [0_u8; IDENTITY_BYTES];
        bytes.copy_from_slice(&hasher.finalize().as_bytes()[..IDENTITY_BYTES]);
        if bytes == [0; IDENTITY_BYTES] {
            bytes[IDENTITY_BYTES - 1] = 1;
        }
        Self(bytes)
    }

    fn parse(value: &str) -> Result<Self, Diagnostic> {
        if value.len() != ENCODED_IDENTITY_BYTES {
            return Err(identity_error(
                DiagnosticClass::Source,
                "semantic_identity_length",
                format!(
                    "identity must contain {ENCODED_IDENTITY_BYTES} lowercase hexadecimal characters"
                ),
            ));
        }
        let mut bytes = [0_u8; IDENTITY_BYTES];
        for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let high = decode_hex(pair[0]).ok_or_else(|| invalid_hex(value))?;
            let low = decode_hex(pair[1]).ok_or_else(|| invalid_hex(value))?;
            bytes[index] = (high << 4) | low;
        }
        if bytes == [0; IDENTITY_BYTES] {
            return Err(identity_error(
                DiagnosticClass::Source,
                "semantic_identity_zero",
                "all-zero semantic identity is reserved",
            ));
        }
        Ok(Self(bytes))
    }

    fn encode(self) -> String {
        encode_hex(&self.0)
    }
}

macro_rules! semantic_id {
    ($name:ident, $prefix:literal, $domain:literal, $tag:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(IdentityBytes);

        impl $name {
            pub const PREFIX: &'static str = $prefix;
            pub const DOMAIN: &'static str = $domain;

            pub fn generate() -> Result<Self, Diagnostic> {
                IdentityBytes::random().map(Self)
            }

            /// Deterministic one-time migration allocation. `ordinal` is a canonical traversal
            /// ordinal, never a path, name, byte position, or content digest identity.
            pub fn migrate(seed: &[u8], ordinal: u64) -> Self {
                Self(IdentityBytes::deterministic(Self::DOMAIN, seed, ordinal))
            }

            /// Exact allocation for one normalized request. The caller-owned seed binds the
            /// repository, base revision, normalized request, and idempotency identity. The
            /// ordinal is the canonical request-local allocation order within this ID domain.
            pub fn allocate(seed: &[u8], ordinal: u64) -> Self {
                Self(IdentityBytes::request_local(Self::DOMAIN, seed, ordinal))
            }

            pub const fn bytes(self) -> [u8; IDENTITY_BYTES] {
                self.0.0
            }

            /// Constructs one typed identity from its exact binary domain payload.
            ///
            /// Stored map keys carry their domain tag separately, so strict key decoders use
            /// this constructor only after checking that tag. The all-zero value remains
            /// reserved in every semantic identity domain.
            pub fn from_bytes(bytes: [u8; IDENTITY_BYTES]) -> Option<Self> {
                if bytes == [0; IDENTITY_BYTES] {
                    None
                } else {
                    Some(Self(IdentityBytes(bytes)))
                }
            }

            pub fn parse(value: &str) -> Result<Self, Diagnostic> {
                value.parse()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(Self::PREFIX)?;
                formatter.write_str(&self.0.encode())
            }
        }

        impl FromStr for $name {
            type Err = Diagnostic;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let encoded = value.strip_prefix(Self::PREFIX).ok_or_else(|| {
                    identity_error(
                        DiagnosticClass::Source,
                        "semantic_identity_domain",
                        format!(
                            "identity belongs to a foreign domain; expected prefix '{}'",
                            Self::PREFIX
                        ),
                    )
                })?;
                IdentityBytes::parse(encoded).map(Self)
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
                let pattern = format!("^{}[0-9a-f]{{32}}$", Self::PREFIX);
                json_schema!({
                    "type": "string",
                    "minLength": Self::PREFIX.len() + 32,
                    "maxLength": Self::PREFIX.len() + 32,
                    "pattern": pattern
                })
            }
        }

        impl Encode for $name {
            fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
                $tag.encode(encoder)?;
                self.0.0.encode(encoder)
            }
        }

        impl<Context> Decode<Context> for $name {
            fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
                let tag = u8::decode(decoder)?;
                if tag != $tag {
                    return Err(DecodeError::OtherString(format!(
                        "foreign semantic identity domain tag {tag}; expected {}",
                        $tag
                    )));
                }
                let bytes = <[u8; IDENTITY_BYTES]>::decode(decoder)?;
                if bytes == [0; IDENTITY_BYTES] {
                    return Err(DecodeError::OtherString(
                        "all-zero semantic identity is reserved".to_owned(),
                    ));
                }
                Ok(Self(IdentityBytes(bytes)))
            }
        }

        bincode::impl_borrow_decode!($name);
    };
}

semantic_id!(RepositoryId, "repo_", "repository", 1u8);
semantic_id!(ModuleId, "mod_", "module", 2u8);
semantic_id!(DeclarationId, "decl_", "declaration", 3u8);
semantic_id!(FieldId, "field_", "record_field", 4u8);
semantic_id!(CaseId, "case_", "variant_case", 5u8);
semantic_id!(OperationId, "op_", "interface_operation", 6u8);
semantic_id!(ParameterId, "param_", "parameter", 7u8);
semantic_id!(BindingId, "bind_", "binding", 8u8);
semantic_id!(ExpressionId, "expr_", "expression_site", 9u8);
semantic_id!(RequirementId, "req_", "capability_requirement", 10u8);
semantic_id!(PortId, "port_", "component_port", 11u8);
semantic_id!(TargetId, "target_", "target", 12u8);
semantic_id!(DraftId, "draft_", "draft", 13u8);
semantic_id!(ConflictId, "conflict_", "conflict", 14u8);
semantic_id!(DocumentationId, "doc_", "documentation", 15u8);
semantic_id!(AnnotationId, "annotation_", "annotation", 16u8);
semantic_id!(TypeParameterId, "typeparam_", "type_parameter", 17u8);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RevisionId([u8; 32]);

impl RevisionId {
    pub const PREFIX: &'static str = "rev_";

    pub const fn from_digest(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn parse(value: &str) -> Result<Self, Diagnostic> {
        value.parse()
    }
}

impl fmt::Display for RevisionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(Self::PREFIX)?;
        formatter.write_str(&encode_hex(&self.0))
    }
}

impl FromStr for RevisionId {
    type Err = Diagnostic;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let encoded = value.strip_prefix(Self::PREFIX).ok_or_else(|| {
            identity_error(
                DiagnosticClass::Source,
                "revision_identity_domain",
                format!("revision identity must start with '{}'", Self::PREFIX),
            )
        })?;
        if encoded.len() != 64 {
            return Err(identity_error(
                DiagnosticClass::Source,
                "revision_identity_length",
                "revision identity must contain 64 lowercase hexadecimal characters",
            ));
        }
        let mut bytes = [0_u8; 32];
        for (index, pair) in encoded.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let high = decode_hex(pair[0]).ok_or_else(|| invalid_hex(encoded))?;
            let low = decode_hex(pair[1]).ok_or_else(|| invalid_hex(encoded))?;
            bytes[index] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

impl Serialize for RevisionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RevisionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for RevisionId {
    fn schema_name() -> Cow<'static, str> {
        "lkjscript.RevisionIdV5".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::RevisionId").into()
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

impl Encode for RevisionId {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        255u8.encode(encoder)?;
        self.0.encode(encoder)
    }
}

impl<Context> Decode<Context> for RevisionId {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let tag = u8::decode(decoder)?;
        if tag != 255 {
            return Err(DecodeError::OtherString(format!(
                "foreign revision identity domain tag {tag}"
            )));
        }
        Ok(Self(<[u8; 32]>::decode(decoder)?))
    }
}

bincode::impl_borrow_decode!(RevisionId);

fn invalid_hex(value: &str) -> Diagnostic {
    identity_error(
        DiagnosticClass::Source,
        "semantic_identity_hex",
        format!("identity '{value}' is not canonical lowercase hexadecimal"),
    )
}

fn decode_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

fn identity_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains_are_explicit_and_migration_is_deterministic() {
        let module = ModuleId::migrate(b"fixture", 7);
        assert_eq!(module, ModuleId::migrate(b"fixture", 7));
        assert_ne!(
            module.bytes(),
            DeclarationId::migrate(b"fixture", 7).bytes()
        );
        assert_eq!(
            module.to_string().parse::<ModuleId>().expect("module"),
            module
        );
        let error = module
            .to_string()
            .parse::<DeclarationId>()
            .expect_err("foreign domain");
        assert_eq!(error.code, "semantic_identity_domain");

        let configuration = bincode::config::standard();
        let encoded = bincode::encode_to_vec(module, configuration).expect("encode module");
        assert!(bincode::decode_from_slice::<DeclarationId, _>(&encoded, configuration).is_err());
    }
}
