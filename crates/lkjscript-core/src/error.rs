//! Shared error type without panics.

use std::fmt;

use crate::ResourceLimitKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub enum ErrorClass {
    Ordinary,
    Deadline,
    Resource(ResourceLimitKind),
    BytecodePolicy,
    Host,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    message: String,
    class: ErrorClass,
}

impl Error {
    pub fn msg(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Ordinary,
        }
    }

    #[doc(hidden)]
    pub fn deadline(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Deadline,
        }
    }

    #[doc(hidden)]
    pub fn resource(kind: ResourceLimitKind, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Resource(kind),
        }
    }

    #[doc(hidden)]
    pub fn bytecode_policy(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::BytecodePolicy,
        }
    }

    #[doc(hidden)]
    pub fn host(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Host,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.message
    }

    #[doc(hidden)]
    pub const fn class(&self) -> ErrorClass {
        self.class
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
