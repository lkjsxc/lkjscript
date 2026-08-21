use serde::Serialize;
use std::fmt;

/// Stable top-level failure ownership. Domain outcomes are language values and never use this
/// classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticClass {
    Source,
    Semantic,
    Capability,
    Resource,
    Cancelled,
    Corrupt,
    Infrastructure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceLocation {
    pub path: String,
    pub byte_offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub class: DiagnosticClass,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<SourceLocation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn new(
        class: DiagnosticClass,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            class,
            code: code.into(),
            message: message.into(),
            location: None,
            notes: Vec::new(),
        }
    }

    pub fn source(
        code: impl Into<String>,
        message: impl Into<String>,
        location: SourceLocation,
    ) -> Self {
        Self {
            class: DiagnosticClass::Source,
            code: code.into(),
            message: message.into(),
            location: Some(location),
            notes: Vec::new(),
        }
    }

    pub fn semantic(
        code: impl Into<String>,
        message: impl Into<String>,
        location: SourceLocation,
    ) -> Self {
        Self {
            class: DiagnosticClass::Semantic,
            code: code.into(),
            message: message.into(),
            location: Some(location),
            notes: Vec::new(),
        }
    }

    pub fn at_end(code: impl Into<String>, message: impl Into<String>, path: &str) -> Self {
        Self::source(
            code,
            message,
            SourceLocation {
                path: path.to_owned(),
                byte_offset: 0,
                line: 1,
                column: 1,
            },
        )
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(location) = &self.location {
            write!(
                formatter,
                "{}:{}:{}: {}: {}",
                location.path, location.line, location.column, self.code, self.message
            )
        } else {
            write!(formatter, "{}: {}", self.code, self.message)
        }
    }
}

impl std::error::Error for Diagnostic {}
