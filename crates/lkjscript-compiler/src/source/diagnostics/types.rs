use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourcePosition {
    pub(crate) byte: u64,
    pub(crate) line: u64,
    pub(crate) column: u64,
}

impl SourcePosition {
    #[cfg(test)]
    pub const fn byte(self) -> u64 {
        self.byte
    }

    pub const fn line(self) -> u64 {
        self.line
    }

    pub const fn column(self) -> u64 {
        self.column
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceSpan {
    pub(crate) start: SourcePosition,
    pub(crate) end: SourcePosition,
}

impl SourceSpan {
    pub const fn start(self) -> SourcePosition {
        self.start
    }

    #[cfg(test)]
    pub const fn end(self) -> SourcePosition {
        self.end
    }

    #[cfg(test)]
    pub const fn byte_range(self) -> std::ops::Range<u64> {
        self.start.byte..self.end.byte
    }

    pub(crate) fn zero() -> Self {
        let position = SourcePosition {
            byte: 0,
            line: 1,
            column: 1,
        };
        Self {
            start: position,
            end: position,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceOrigin {
    pub(crate) logical_path: String,
    pub(crate) host_containment_path: Option<PathBuf>,
}

impl SourceOrigin {
    pub fn logical_path(&self) -> &str {
        &self.logical_path
    }

    /// Canonical host path after containment checking. In-memory source has no
    /// host path and still has an independent canonical logical origin.
    #[cfg(test)]
    pub fn host_containment_path(&self) -> Option<&Path> {
        self.host_containment_path.as_deref()
    }

    pub(crate) fn in_memory(path: &str) -> Self {
        Self {
            logical_path: crate::source::validate::canonical_logical_path(Path::new(path)),
            host_containment_path: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
}

impl DiagnosticSeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticCategory {
    SourceSyntax,
    Declaration,
    SourceLoading,
}

impl DiagnosticCategory {
    #[cfg(test)]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceSyntax => "source-syntax",
            Self::Declaration => "declaration",
            Self::SourceLoading => "source-loading",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelatedSourceSpan {
    pub(super) label: String,
    pub(super) origin: SourceOrigin,
    pub(super) span: SourceSpan,
}
