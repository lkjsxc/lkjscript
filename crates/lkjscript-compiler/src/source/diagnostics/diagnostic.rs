use std::fmt;

use lkjscript_core::Error;

use super::{DiagnosticCategory, DiagnosticSeverity, RelatedSourceSpan, SourceOrigin, SourceSpan};

/// Structured source-foundation diagnostic rendered by compiler entry points.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDiagnostic {
    code: &'static str,
    severity: DiagnosticSeverity,
    category: DiagnosticCategory,
    message: String,
    origin: Box<SourceOrigin>,
    primary_span: Option<Box<SourceSpan>>,
    related: Vec<RelatedSourceSpan>,
}

impl SourceDiagnostic {
    pub const fn code(&self) -> &'static str {
        self.code
    }

    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    pub const fn category(&self) -> DiagnosticCategory {
        self.category
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn origin(&self) -> Option<&SourceOrigin> {
        self.primary_span.as_ref().map(|_| self.origin.as_ref())
    }

    pub fn primary_span(&self) -> Option<SourceSpan> {
        self.primary_span.as_deref().copied()
    }

    pub fn related_spans(&self) -> &[RelatedSourceSpan] {
        &self.related
    }
    pub fn render_human(&self) -> String {
        let mut rendered = self.primary_span.as_deref().map_or_else(
            || {
                format!(
                    "{}[{}]: {}",
                    self.severity.as_str(),
                    self.code,
                    self.message
                )
            },
            |span| {
                let start = span.start();
                format!(
                    "{}:{}:{}: {}[{}]: {}",
                    self.origin.logical_path(),
                    start.line(),
                    start.column(),
                    self.severity.as_str(),
                    self.code,
                    self.message
                )
            },
        );
        for related in &self.related {
            match related.span {
                Some(span) => {
                    let start = span.start();
                    rendered.push_str(&format!(
                        "\n  related {}:{}:{}: {}",
                        related.origin.logical_path(),
                        start.line(),
                        start.column(),
                        related.label
                    ));
                }
                None => rendered.push_str(&format!("\n  related: {}", related.label)),
            }
        }
        rendered
    }
    pub(crate) fn new(
        code: &'static str,
        category: DiagnosticCategory,
        message: impl Into<String>,
        origin: SourceOrigin,
        primary_span: SourceSpan,
    ) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Error,
            category,
            message: message.into(),
            origin: Box::new(origin),
            primary_span: if primary_span.is_zero() {
                None
            } else {
                Some(Box::new(primary_span))
            },
            related: Vec::new(),
        }
    }
    pub(crate) fn with_related(
        mut self,
        label: impl Into<String>,
        origin: SourceOrigin,
        span: SourceSpan,
    ) -> Self {
        self.related.push(RelatedSourceSpan {
            label: label.into(),
            origin,
            span: (!span.is_zero()).then_some(span),
        });
        self
    }
    pub(crate) fn generic(origin: SourceOrigin, message: impl Into<String>) -> Self {
        Self::new(
            "LKJ-SRC-INVALID",
            DiagnosticCategory::SourceSyntax,
            message,
            origin,
            SourceSpan::zero(),
        )
    }
    pub(crate) fn loading(origin: SourceOrigin, message: impl Into<String>) -> Self {
        Self::new(
            "LKJ-SRC-LOAD",
            DiagnosticCategory::SourceLoading,
            message,
            origin,
            SourceSpan::zero(),
        )
    }

    pub(crate) fn host(origin: SourceOrigin, message: impl Into<String>) -> Self {
        Self::new(
            "LKJ-SRC-HOST",
            DiagnosticCategory::SourceLoading,
            message,
            origin,
            SourceSpan::zero(),
        )
    }

    pub(crate) fn into_core(self) -> Error {
        let rendered = self.render_human();
        if self.code == "LKJ-SRC-HOST" {
            Error::host(rendered)
        } else {
            Error::msg(rendered)
        }
    }
}

impl fmt::Display for SourceDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.render_human())
    }
}

impl std::error::Error for SourceDiagnostic {}

pub type SourceResult<T> = std::result::Result<T, SourceDiagnostic>;
