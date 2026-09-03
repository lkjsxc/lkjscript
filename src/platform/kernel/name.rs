//! Explicit canonical Graph 9 name value.

use super::contract::MAXIMUM_NAME_BYTES;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{BorrowDecode, Decode, Encode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Borrow;
use std::fmt;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Name(String);

impl Name {
    pub fn new(value: impl Into<String>) -> Result<Self, Diagnostic> {
        let value = value.into();
        validate_name(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Borrow<str> for Name {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Serialize for Name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl Encode for Name {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.0.encode(encoder)
    }
}

impl<Context> Decode<Context> for Name {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let value = String::decode(decoder)?;
        Self::new(value).map_err(|error| DecodeError::OtherString(error.message))
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for Name {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Self::decode(decoder)
    }
}

fn validate_name(value: &str) -> Result<(), Diagnostic> {
    if value.is_empty() || value.len() > MAXIMUM_NAME_BYTES {
        return Err(name_error(format!(
            "name must contain 1 through {MAXIMUM_NAME_BYTES} bytes"
        )));
    }
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return Err(name_error("name is empty"));
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return Err(name_error(
            "name must start with an ASCII letter or underscore",
        ));
    }
    if !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-') {
        return Err(name_error(
            "name contains a character outside [A-Za-z0-9_-]",
        ));
    }
    Ok(())
}

fn name_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, "kernel_name", message)
}
