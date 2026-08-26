//! Bounded, canonical host implementation locators.

use super::contract::MAXIMUM_NAME_BYTES;
use super::name::Name;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::{BorrowDecode, Decode, Encode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A dot-separated host implementation locator. Semantic owner names remain single `Name`
/// values; external implementations use this distinct type because maintained intrinsic
/// locators such as `core.text.concat` are hierarchical and are not namespace owners.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ImplementationName(String);

impl ImplementationName {
    pub fn new(value: impl Into<String>) -> Result<Self, Diagnostic> {
        let value = value.into();
        if value.is_empty() || value.len() > MAXIMUM_NAME_BYTES {
            return Err(implementation_error(format!(
                "implementation locator must contain 1 through {MAXIMUM_NAME_BYTES} bytes"
            )));
        }
        for segment in value.split('.') {
            Name::new(segment.to_owned()).map_err(|_| {
                implementation_error(
                    "implementation locator must contain canonical dot-separated name segments",
                )
            })?;
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ImplementationName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for ImplementationName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ImplementationName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl Encode for ImplementationName {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        self.0.encode(encoder)
    }
}

impl<Context> Decode<Context> for ImplementationName {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let value = String::decode(decoder)?;
        Self::new(value).map_err(|error| bincode::error::DecodeError::OtherString(error.message))
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for ImplementationName {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        Self::decode(decoder)
    }
}

fn implementation_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "kernel_implementation_name",
        message,
    )
}
