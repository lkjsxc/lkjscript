mod diagnostic;
mod types;

pub use diagnostic::{SourceDiagnostic, SourceResult};
pub use types::{
    DiagnosticCategory, DiagnosticSeverity, RelatedSourceSpan, SourceOrigin, SourcePosition,
    SourceSpan,
};
