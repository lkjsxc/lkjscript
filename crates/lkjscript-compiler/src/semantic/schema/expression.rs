use serde::{Deserialize, Serialize};

use crate::source::{SourceNode, SourceSpan, SyntaxKind};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Expression {
    Unit,
    Bool {
        value: bool,
    },
    I64 {
        value: i64,
    },
    F64 {
        value: String,
    },
    String {
        value: String,
    },
    Symbol {
        name: String,
    },
    Call {
        name: String,
        children: Vec<Expression>,
    },
}

impl Expression {
    pub(crate) fn measure(&self, depth: u32, counts: &mut ExpressionCounts) {
        counts.nodes = counts.nodes.saturating_add(1);
        counts.depth = counts.depth.max(depth);
        match self {
            Self::F64 { value } | Self::String { value } | Self::Symbol { name: value } => {
                counts.string_bytes = counts.string_bytes.saturating_add(value.len() as u64);
            }
            Self::Call { name, children } => {
                counts.string_bytes = counts.string_bytes.saturating_add(name.len() as u64);
                for child in children {
                    child.measure(depth.saturating_add(1), counts);
                }
            }
            Self::Unit | Self::Bool { .. } | Self::I64 { .. } => {}
        }
    }

    pub(crate) fn to_source(&self, span: SourceSpan) -> Result<SourceNode, String> {
        let (kind, children) = match self {
            Self::Unit => (SyntaxKind::Unit, Vec::new()),
            Self::Bool { value } => (SyntaxKind::Bool { value: *value }, Vec::new()),
            Self::I64 { value } => (SyntaxKind::I64 { value: *value }, Vec::new()),
            Self::F64 { value } => {
                let parsed = value
                    .parse::<f64>()
                    .map_err(|_| format!("invalid canonical F64 value {value:?}"))?;
                if !parsed.is_finite() || crate::source::format_f64(parsed) != *value {
                    return Err(format!("non-canonical or non-finite F64 value {value:?}"));
                }
                (SyntaxKind::F64 { value: parsed }, Vec::new())
            }
            Self::String { value } => (
                SyntaxKind::Str {
                    value: value.clone(),
                },
                Vec::new(),
            ),
            Self::Symbol { name } => {
                if !crate::source::is_source_identifier(name) {
                    return Err(format!("invalid source symbol {name:?}"));
                }
                (SyntaxKind::Symbol { name: name.clone() }, Vec::new())
            }
            Self::Call { name, children } => {
                if !crate::source::is_source_identifier(name) {
                    return Err(format!("invalid source call name {name:?}"));
                }
                let children = children
                    .iter()
                    .map(|child| child.to_source(span))
                    .collect::<Result<Vec<_>, _>>()?;
                (SyntaxKind::Call { name: name.clone() }, children)
            }
        };
        Ok(SourceNode {
            kind,
            span,
            leading_trivia: Vec::new(),
            before_close_trivia: Vec::new(),
            children,
        })
    }
}

#[derive(Default)]
pub(crate) struct ExpressionCounts {
    pub nodes: u64,
    pub depth: u32,
    pub string_bytes: u64,
}
