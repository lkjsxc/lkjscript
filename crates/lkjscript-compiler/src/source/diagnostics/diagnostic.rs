use std::fmt;

use lkjscript_core::Error;

use super::{
    DiagnosticCategory, DiagnosticCertainty, DiagnosticSeverity, RelatedSourceSpan, SourceOrigin,
    SourceSpan,
};
use crate::source::{
    SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA, SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA_VERSION,
};

/// Structured source-foundation diagnostic rendered by compiler entry points.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDiagnostic {
    code: &'static str,
    severity: DiagnosticSeverity,
    category: DiagnosticCategory,
    certainty: DiagnosticCertainty,
    message: String,
    origin: Box<SourceOrigin>,
    primary_span: SourceSpan,
    related: Vec<RelatedSourceSpan>,
    budget: Option<Box<lkjscript_core::BudgetError>>,
}

impl SourceDiagnostic {
    pub const fn schema(&self) -> &'static str {
        SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA
    }
    pub const fn schema_version(&self) -> u32 {
        SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA_VERSION
    }
    pub const fn code(&self) -> &'static str {
        self.code
    }
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }
    pub const fn category(&self) -> DiagnosticCategory {
        self.category
    }
    pub const fn certainty(&self) -> DiagnosticCertainty {
        self.certainty
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn origin(&self) -> &SourceOrigin {
        self.origin.as_ref()
    }
    pub const fn primary_span(&self) -> SourceSpan {
        self.primary_span
    }
    pub fn related_spans(&self) -> &[RelatedSourceSpan] {
        &self.related
    }
    pub fn budget_error(&self) -> Option<&lkjscript_core::BudgetError> {
        self.budget.as_deref()
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
    pub fn render_compact_agent(&self) -> String {
        let start = self.primary_span.start();
        let end = self.primary_span.end();
        let mut rendered = format!(
            concat!(
                "schema={};version={};",
                "code={};severity={};category={};certainty={};origin={};",
                "span={}:{}:{}-{}:{}:{};message={}"
            ),
            SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA,
            SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA_VERSION,
            self.code,
            self.severity.as_str(),
            self.category.as_str(),
            self.certainty.as_str(),
            crate::source::identity::escape_compact(self.origin.logical_path()),
            start.byte(),
            start.line(),
            start.column(),
            end.byte(),
            end.line(),
            end.column(),
            crate::source::identity::escape_compact(&self.message)
        );
        for (index, related) in self.related.iter().enumerate() {
            let start = related.span.start();
            let end = related.span.end();
            rendered.push_str(&format!(
                concat!(
                    ";related[{index}].label={};related[{index}].origin={};",
                    "related[{index}].span={}:{}:{}-{}:{}:{}"
                ),
                crate::source::identity::escape_compact(&related.label),
                crate::source::identity::escape_compact(related.origin.logical_path()),
                start.byte(),
                start.line(),
                start.column(),
                end.byte(),
                end.line(),
                end.column(),
                index = index
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
        Self {
            code,
            severity: DiagnosticSeverity::Error,
            category,
            certainty: DiagnosticCertainty::Guaranteed,
            message: message.into(),
            origin: Box::new(origin),
            primary_span,
            related: Vec::new(),
            budget: None,
        }
    }
    pub(crate) fn with_budget(mut self, failure: lkjscript_core::BudgetError) -> Self {
        self.budget = Some(Box::new(failure));
        self
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

    pub(crate) fn into_core(self) -> Error {
        self.budget
            .as_deref()
            .copied()
            .map_or_else(|| Error::msg(self.render_human()), Error::budget)
    }
}

impl fmt::Display for SourceDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.render_human())
    }
}

impl std::error::Error for SourceDiagnostic {}

pub type SourceResult<T> = std::result::Result<T, SourceDiagnostic>;
