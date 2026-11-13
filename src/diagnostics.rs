//! Reporting data does not carry checked K and cannot authorize emission.
use crate::ast::{Span, Type};
use crate::source::SourceReport;
use std::collections::{BTreeMap, BTreeSet};

pub struct BlockedRegion {
    pub path: String,
    pub span: Span,
    pub kind: String,
    pub reason: String,
}

#[derive(Default)]
pub struct Census {
    pub unsupported: BTreeMap<String, usize>,
    pub unsupported_sites: Vec<(String, Span)>,
    pub blocked: BTreeMap<String, usize>,
}

fn count(counts: &mut BTreeMap<String, usize>, kind: &str) {
    *counts.entry(kind.to_string()).or_insert(0) += 1;
}

/// Count recognized constructs whose lowering is unconditionally unavailable.
/// Type-dependent method selection and inference failures require semantic
/// analysis and are not guessed from a method's spelling. Skipped tokens remain
/// blocked regions, so an absent count never claims that unseen syntax is absent.
pub fn census(report: &SourceReport) -> Census {
    let mut result = Census::default();
    // A semantic error can name an already counted syntax node. Retain its
    // diagnostic for the caller, but count the unsupported source site once.
    let mut recognized = Vec::new();
    let mut sites = BTreeSet::new();
    for region in &report.blocked {
        count(&mut result.blocked, &region.kind);
    }
    let program = &report.program;
    for node in &program.types {
        if matches!(node.kind, Type::Function { .. }) {
            recognized.push(("function pointer", node.span));
        }
    }
    for (kind, span) in recognized {
        count(&mut result.unsupported, kind);
        result.unsupported_sites.push((kind.to_string(), span));
        sites.insert((span.file, span.line, span.column));
    }
    for problem in &report.diagnostics {
        let kind = if problem.code == "E_UNSUPPORTED" {
            problem
                .message
                .strip_prefix("unsupported construct: ")
                .unwrap_or(&problem.message)
        } else if problem.code == "E_UNSUPPORTED_MACRO" {
            problem
                .message
                .strip_prefix("unsupported macro: ")
                .unwrap_or(&problem.message)
        } else {
            continue;
        };
        if sites.insert((problem.file, problem.line, problem.column)) {
            count(&mut result.unsupported, kind);
        }
    }
    result
}

/// Report every recognized unsupported site, including those after the first
/// failed body. Existing diagnostics retain their more specific explanation.
pub fn complete_unsupported(report: &mut SourceReport) {
    let sites = census(report).unsupported_sites;
    let mut seen: BTreeSet<_> = report
        .diagnostics
        .iter()
        .map(|p| (p.file, p.line, p.column))
        .collect();
    for (kind, span) in sites {
        if seen.insert((span.file, span.line, span.column)) {
            let mut problem = crate::ast::Diagnostic::new(
                span,
                "E_UNSUPPORTED",
                &format!("unsupported construct: {kind}"),
            );
            if let Some(file) = report.program.files.get(span.file) {
                problem.path = file.path.clone();
            }
            report.diagnostics.push(problem);
        }
    }
}

/// Actual resolution/type/trait diagnostics, not a guess based on a method name.
/// A blocked body contributes no claimed result for its unresolved methods.
pub fn semantic_census(report: &SourceReport) -> BTreeMap<String, usize> {
    let mut result = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let problems = crate::resolve::resolution_problems(&report.program);
    for problem in problems.iter().chain(&report.diagnostics) {
        let semantic_unsupported = problem.code == "E_UNSUPPORTED"
            && ["trait", "method", "import", "lifetime"]
                .iter()
                .any(|word| problem.message.contains(word));
        if !semantic_unsupported
            && !matches!(
                problem.code.as_str(),
                "E_NAME" | "E_IMPORT" | "E_METHOD" | "E_TRAIT" | "E_GENERIC" | "E_TYPE"
            )
        {
            continue;
        }
        if seen.insert((
            problem.file,
            problem.line,
            problem.column,
            problem.code.clone(),
            problem.message.clone(),
        )) {
            count(
                &mut result,
                &format!("{} {}", problem.code, problem.message),
            );
        }
    }
    result
}
