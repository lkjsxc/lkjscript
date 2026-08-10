use std::fmt;

use lkjscript_core::Error;

use crate::SourceDiagnostic;

/// Failure at the local package boundary.
///
/// Source loading and parsing keep their existing structured diagnostic. All
/// other package facts retain the core error and its broad class.
#[derive(Debug)]
pub enum PackageError {
    Source(SourceDiagnostic),
    Package(Error),
}

impl From<SourceDiagnostic> for PackageError {
    fn from(error: SourceDiagnostic) -> Self {
        Self::Source(error)
    }
}

impl From<Error> for PackageError {
    fn from(error: Error) -> Self {
        Self::Package(error)
    }
}

impl fmt::Display for PackageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => error.fmt(formatter),
            Self::Package(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PackageError {}

pub type PackageResult<T> = std::result::Result<T, PackageError>;
