//! Shared error type without panics.

use std::fmt;

use crate::{BudgetError, ResourceDiagnostic, ResourceLimitKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub enum ErrorClass {
    Ordinary,
    Deadline,
    Resource(ResourceLimitKind),
    Host,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    message: String,
    class: ErrorClass,
    compiler_resource: Option<Box<ResourceDiagnostic>>,
    budget: Option<Box<BudgetError>>,
}

impl Error {
    pub fn msg(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Ordinary,
            compiler_resource: None,
            budget: None,
        }
    }

    #[doc(hidden)]
    pub fn deadline(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Deadline,
            compiler_resource: None,
            budget: None,
        }
    }

    #[doc(hidden)]
    pub fn resource(kind: ResourceLimitKind, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Resource(kind),
            compiler_resource: None,
            budget: None,
        }
    }

    #[doc(hidden)]
    pub fn host(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Host,
            compiler_resource: None,
            budget: None,
        }
    }

    pub fn compiler_resource(diagnostic: ResourceDiagnostic) -> Self {
        Self {
            message: diagnostic.to_string(),
            class: ErrorClass::Ordinary,
            compiler_resource: Some(Box::new(diagnostic)),
            budget: None,
        }
    }

    pub fn budget(error: BudgetError) -> Self {
        Self {
            message: error.to_string(),
            class: ErrorClass::Ordinary,
            compiler_resource: None,
            budget: Some(Box::new(error)),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.message
    }

    pub fn compiler_resource_diagnostic(&self) -> Option<&ResourceDiagnostic> {
        self.compiler_resource.as_deref()
    }

    pub fn budget_error(&self) -> Option<&BudgetError> {
        self.budget.as_deref()
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
