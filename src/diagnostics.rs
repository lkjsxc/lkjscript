//! Reporting data does not carry checked K and cannot authorize emission.
use crate::ast::{Expr, Pattern, Span, Type};
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
    for node in &program.exprs {
        let kind = match &node.kind {
            Expr::IfLet { .. } => Some("if let"),
            Expr::WhileLet { .. } => Some("while let"),
            Expr::Try(_) => Some("?"),
            Expr::Closure { .. } => Some("closure"),
            Expr::Break(Some(_)) => Some("break value"),
            Expr::Char(_) => Some("char literal"),
            Expr::Struct { base: Some(_), .. } => Some("struct update"),
            Expr::For {
                iterator, pattern, ..
            } => {
                if !matches!(program.exprs[*iterator].kind, Expr::Range { .. }) {
                    recognized.push(("collection iteration", program.exprs[*iterator].span));
                    sites.insert((node.span.file, node.span.line, node.span.column));
                }
                if !matches!(program.patterns[*pattern].kind, Pattern::Bind { .. }) {
                    recognized.push(("for binding pattern", program.patterns[*pattern].span));
                    sites.insert((node.span.file, node.span.line, node.span.column));
                }
                None
            }
            Expr::Index { index, .. }
                if matches!(program.exprs[*index].kind, Expr::Range { .. }) =>
            {
                let span = program.exprs[*index].span;
                sites.insert((span.file, span.line, span.column));
                Some("range slicing")
            }
            Expr::Macro {
                name, separator, ..
            } if name == "vec" && separator == ";" => Some("repeated Vec literal"),
            Expr::Macro { name, .. } if name == "format" => Some("format!"),
            Expr::Match { arms, .. } => {
                for arm in arms {
                    if arm.guard.is_some() {
                        recognized.push(("match guard", arm.span));
                    }
                }
                None
            }
            _ => None,
        };
        if let Some(kind) = kind {
            recognized.push((kind, node.span));
        }
    }
    for node in &program.patterns {
        match &node.kind {
            Pattern::Or(_) => recognized.push(("or-pattern", node.span)),
            Pattern::Ref { .. } => recognized.push(("reference pattern", node.span)),
            _ => {}
        }
    }
    for node in &program.types {
        if matches!(node.kind, Type::Function { .. }) {
            recognized.push(("function pointer", node.span));
        }
    }
    for (kind, span) in recognized {
        count(&mut result.unsupported, kind);
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
