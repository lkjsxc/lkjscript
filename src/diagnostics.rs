//! Reporting data does not carry checked IR and cannot authorize emission.
pub(crate) mod lookup_domains;
pub(crate) mod readiness;
use crate::ast::{Span, Type};
use crate::source::SourceReport;
use std::collections::{BTreeMap, BTreeSet};

/// Facts consumed before a declaration failed to parse. These only restrict
/// diagnostic lookup; they never create a declaration or an executable type.
#[derive(Clone)]
pub enum RecoveryOwner {
    Implementation {
        target: crate::ast::TypeId,
        trait_ty: Option<crate::ast::TypeId>,
    },
    Trait {
        name: String,
    },
}
#[derive(Clone)]
pub struct RecoveryDomain {
    pub scope: crate::ast::ModuleId,
    pub kind: String,
    pub name: Option<String>,
    pub owner: Option<RecoveryOwner>,
}

#[derive(Clone)]
pub struct BlockedRegion {
    pub path: String,
    pub span: Span,
    pub kind: String,
    pub reason: String,
    pub domain: Option<RecoveryDomain>,
}

#[derive(Default)]
pub struct Census {
    pub unsupported: BTreeMap<String, usize>,
    pub unsupported_sites: Vec<(String, Span)>,
    pub blocked: BTreeMap<String, usize>,
}

pub(crate) fn add_count(counts: &mut BTreeMap<String, usize>, kind: &str, amount: usize) {
    let key = kind.to_string();
    let previous = counts.get(&key).copied().unwrap_or(0);
    counts.insert(key, previous + amount);
}
pub(crate) fn count(counts: &mut BTreeMap<String, usize>, kind: &str) {
    add_count(counts, kind, 1);
}

/// Count recognized constructs whose lowering is unconditionally unavailable.
/// Type-dependent method selection and inference failures require semantic
/// analysis and are not guessed from a method's spelling. Skipped tokens remain
/// blocked regions, so an absent count never claims that unseen syntax is absent.
pub fn census(report: &SourceReport) -> Census {
    let mut result = Census::default();
    // A semantic error can name an already counted syntax node. Retain its
    // diagnostic for the caller, but count the unsupported source site once.
    let mut recognized: Vec<Span> = Vec::new();
    let mut sites: BTreeSet<(usize, usize, usize)> = BTreeSet::new();
    for region in &report.blocked {
        count(&mut result.blocked, &region.kind);
    }
    let program = &report.program;
    for node in &program.types {
        if matches!(node.kind, Type::Function { .. }) {
            recognized.push(node.span);
        }
    }
    for span in recognized {
        let kind = "function pointer";
        count(&mut result.unsupported, kind);
        result.unsupported_sites.push((kind.to_string(), span));
        sites.insert((span.file, span.line, span.column));
    }
    for problem in &report.diagnostics {
        let kind = if problem.code == "E_UNSUPPORTED" {
            problem
                .message
                .as_str()
                .strip_prefix("unsupported construct: ")
                .unwrap_or(problem.message.as_str())
        } else if problem.code == "E_UNSUPPORTED_MACRO" {
            problem
                .message
                .as_str()
                .strip_prefix("unsupported macro: ")
                .unwrap_or(problem.message.as_str())
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
    let mut seen: BTreeSet<(usize, usize, usize)> = BTreeSet::new();
    for problem in &report.diagnostics {
        seen.insert((problem.file, problem.line, problem.column));
    }
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

/// A diagnostic observation carries source locations, never inference or IR evidence.
#[derive(Clone)]
pub struct SemanticObservation {
    pub function: Option<String>,
    pub phase: String,
    pub problem: crate::ast::Diagnostic,
    pub receiver: Option<String>,
    pub member: Option<String>,
}
#[derive(Default)]
pub struct SemanticReport {
    pub observations: Vec<SemanticObservation>,
    pub blocked: Vec<SemanticObservation>,
    pub coverage: BTreeMap<String, usize>,
}
impl SemanticReport {
    pub fn counts(&self) -> BTreeMap<String, usize> {
        let mut result = BTreeMap::new();
        for observation in &self.observations {
            if semantic_problem(&observation.problem) {
                let detail = match (&observation.receiver, &observation.member) {
                    (Some(receiver), Some(member)) => format!(" [{receiver}::{member}]"),
                    _ => String::new(),
                };
                count(
                    &mut result,
                    &format!(
                        "{} {}{detail}",
                        observation.problem.code, observation.problem.message
                    ),
                );
            }
        }
        result
    }
    pub fn blocked_counts(&self) -> BTreeMap<String, usize> {
        let mut result = BTreeMap::new();
        for observation in &self.blocked {
            count(&mut result, &observation.phase);
        }
        result
    }
    pub(crate) fn deduplicate(&mut self) {
        let mut seen: BTreeSet<(usize, usize, usize, String, String)> = BTreeSet::new();
        self.observations.retain(|observation| {
            seen.insert((
                observation.problem.file,
                observation.problem.line,
                observation.problem.column,
                observation.problem.code.clone(),
                observation.problem.message.clone(),
            ))
        });
        let mut seen: BTreeSet<(Option<String>, String, usize, usize, usize, String)> =
            BTreeSet::new();
        self.blocked.retain(|observation| {
            seen.insert((
                observation.function.clone(),
                observation.phase.clone(),
                observation.problem.file,
                observation.problem.line,
                observation.problem.column,
                observation.problem.message.clone(),
            ))
        });
    }
}
pub(crate) fn semantic_problem(problem: &crate::ast::Diagnostic) -> bool {
    (problem.code == "E_UNSUPPORTED"
        && (problem.message.contains("trait")
            || problem.message.contains("method")
            || problem.message.contains("import")
            || problem.message.contains("lifetime")))
        || matches!(
            problem.code.as_str(),
            "E_NAME"
                | "E_IMPORT"
                | "E_METHOD"
                | "E_TRAIT"
                | "E_GENERIC"
                | "E_TYPE"
                | "E_BORROW"
                | "E_MOVE"
        )
}
/// Analyze independent available source facts. The result cannot authorize emission.
pub fn semantic_report(report: &SourceReport) -> SemanticReport {
    crate::elaborate::census::report(report)
}
/// Actual resolution/type/trait diagnostics, including receiver-qualified method
/// failures. Incomplete lookups are available separately in semantic_report.
pub fn semantic_census(report: &SourceReport) -> BTreeMap<String, usize> {
    semantic_report(report).counts()
}
