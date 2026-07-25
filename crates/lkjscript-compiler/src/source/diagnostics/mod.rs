mod diagnostic;
mod types;

pub use diagnostic::{SourceDiagnostic, SourceResult};
pub use types::{
    DiagnosticCategory, DiagnosticCertainty, DiagnosticSeverity, RelatedSourceSpan, SourceOrigin,
    SourcePosition, SourceSpan,
};
