//! Reporting data does not carry checked IR and cannot authorize emission.
use crate::ast::{Span, Type};
use crate::source::SourceReport;
use std::collections::BTreeSet;

#[derive(Clone)]
pub struct BlockedRegion {
    pub path: String,
    pub span: Span,
    pub kind: String,
    pub reason: String,
}

/// Report every recognized unsupported site, including those after the first
/// failed body. Existing diagnostics retain their more specific explanation.
pub fn complete_unsupported(report: &mut SourceReport) {
    let mut seen: BTreeSet<(usize, usize, usize)> = BTreeSet::new();
    for problem in &report.diagnostics {
        seen.insert((problem.file, problem.line, problem.column));
    }
    for node in &report.program.types {
        let span = node.span;
        if matches!(node.kind, Type::Function { .. })
            && seen.insert((span.file, span.line, span.column))
        {
            let mut problem = crate::ast::Diagnostic::new(
                span,
                "E_UNSUPPORTED",
                "unsupported construct: function pointer",
            );
            if let Some(file) = report.program.files.get(span.file) {
                problem.path = file.path.clone();
            }
            report.diagnostics.push(problem);
        }
    }
}
