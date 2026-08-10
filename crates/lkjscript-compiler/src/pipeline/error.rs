use std::fmt;

use lkjscript_core::Error;

use crate::package::PackageError;
use crate::{CompileSnapshotError, IncompleteSnapshotError, SourceDiagnostic};

/// One failure from required-package import through validated bytecode.
///
/// Compilation is fail-fast, so this type carries exactly one current
/// failure. It preserves structured source and completeness facts while
/// leaving package and later compiler errors in their honest broad form.
#[derive(Debug)]
pub enum PackageCompileError {
    Source(SourceDiagnostic),
    Package(Error),
    Incomplete(IncompleteSnapshotError),
    Compiler(Error),
}

impl PackageCompileError {
    pub(crate) fn into_core(self) -> Error {
        match self {
            Self::Source(error) => error.into_core(),
            Self::Package(error) | Self::Compiler(error) => error,
            Self::Incomplete(error) => Error::msg(error.to_string()),
        }
    }
}

impl From<PackageError> for PackageCompileError {
    fn from(error: PackageError) -> Self {
        match error {
            PackageError::Source(error) => Self::Source(error),
            PackageError::Package(error) => Self::Package(error),
        }
    }
}

impl From<CompileSnapshotError> for PackageCompileError {
    fn from(error: CompileSnapshotError) -> Self {
        match error {
            CompileSnapshotError::Incomplete(error) => Self::Incomplete(error),
            CompileSnapshotError::Package(error) => Self::Package(error),
            CompileSnapshotError::Compiler(error) => Self::Compiler(error),
        }
    }
}

impl fmt::Display for PackageCompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => error.fmt(formatter),
            Self::Package(error) => write!(formatter, "package: {error}"),
            Self::Incomplete(error) => error.fmt(formatter),
            Self::Compiler(error) => write!(formatter, "compiler: {error}"),
        }
    }
}

impl std::error::Error for PackageCompileError {}

pub type PackageCompileResult<T> = std::result::Result<T, PackageCompileError>;
