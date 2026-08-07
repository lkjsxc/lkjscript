use std::fmt;

use lkjscript_core::Error;

use super::{DiagnosticCategory, DiagnosticSeverity, RelatedSourceSpan, SourceOrigin, SourceSpan};
/// Structured source-foundation diagnostic rendered by compiler entry points.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDiagnostic {
    code: &'static str,
    severity: DiagnosticSeverity,
    #[cfg(test)]
    category: DiagnosticCategory,
    message: String,
    origin: Box<SourceOrigin>,
    primary_span: Box<SourceSpan>,
    related: Vec<RelatedSourceSpan>,
}

impl SourceDiagnostic {
    #[cfg(test)]
    pub const fn code(&self) -> &'static str {
        self.code
    }
    #[cfg(test)]
    pub const fn category(&self) -> DiagnosticCategory {
        self.category
    }
    #[cfg(test)]
    pub fn message(&self) -> &str {
        &self.message
    }
    #[cfg(test)]
    pub fn primary_span(&self) -> SourceSpan {
        *self.primary_span
    }
    #[cfg(test)]
    pub fn related_spans(&self) -> &[RelatedSourceSpan] {
        &self.related
    }
    pub fn render_human(&self) -> String {
        let start = self.primary_span.start();
        let mut rendered = format!(
            "{}:{}:{}: {}[{}]: {}",
            self.origin.logical_path(),
            start.line(),
            start.column(),
            self.severity.as_str(),
            self.code,
            self.message
        );
        for related in &self.related {
            let start = related.span.start();
            rendered.push_str(&format!(
                "\n  related {}:{}:{}: {}",
                related.origin.logical_path(),
                start.line(),
                start.column(),
                related.label
            ));
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
        #[cfg(not(test))]
        let _ = category;
        Self {
            code,
            severity: DiagnosticSeverity::Error,
            #[cfg(test)]
            category,
            message: message.into(),
            origin: Box::new(origin),
            primary_span: Box::new(primary_span),
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
            span,
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
