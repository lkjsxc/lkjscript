use serde::{Deserialize, Serialize};

use crate::source::{SourceNode, SourceSpan, SyntaxKind};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TypeExpression {
    Never {},
    Unit {},
    Bool {},
    I64 {},
    F64 {},
    String {},
    Buffer {},
    Path {},
    Capability {
        capability: String,
    },
    Symbol {},
    Handle {},
    Product {
        name: String,
    },
    Enum {
        name: String,
        arguments: Vec<TypeExpression>,
    },
    Variable {
        name: String,
    },
    Owned {
        inner: Box<TypeExpression>,
    },
    Ref {
        inner: Box<TypeExpression>,
    },
    RefMut {
        inner: Box<TypeExpression>,
    },
    List {
        element: Box<TypeExpression>,
    },
    Option {
        value: Box<TypeExpression>,
    },
    Result {
        ok: Box<TypeExpression>,
        error: Box<TypeExpression>,
    },
}

impl TypeExpression {
    pub(crate) fn to_atoms(&self, span: SourceSpan) -> Result<Vec<SourceNode>, String> {
        let mut output = Vec::new();
        self.collect_nodes(span, &mut output)?;
        Ok(output)
    }

    fn collect_nodes(&self, span: SourceSpan, output: &mut Vec<SourceNode>) -> Result<(), String> {
        match self {
            Self::Never {} => output.push(atom("Never".into(), span)),
            Self::Unit {} => output.push(atom("Unit".into(), span)),
            Self::Bool {} => output.push(atom("Bool".into(), span)),
            Self::I64 {} => output.push(atom("I64".into(), span)),
            Self::F64 {} => output.push(atom("F64".into(), span)),
            Self::String {} => output.push(atom("Str".into(), span)),
            Self::Buffer {} => output.push(atom("Buf".into(), span)),
            Self::Path {} => output.push(atom("Path".into(), span)),
            Self::Capability { capability } => {
                if lkjscript_core::CapabilityKind::parse(capability).is_none() {
                    return Err(format!("unknown capability kind {capability}"));
                }
                output.push(call(
                    "Capability",
                    vec![atom(capability.clone(), span)],
                    span,
                ));
            }
            Self::Symbol {} => output.push(atom("Symbol".into(), span)),
            Self::Handle {} => output.push(atom("Handle".into(), span)),
            Self::Product { name } => {
                validate_type_name(name, "product")?;
                output.extend([atom("Product".into(), span), atom(name.clone(), span)]);
            }
            Self::Enum { name, arguments } => {
                validate_type_name(name, "enum")?;
                let mut children = Vec::new();
                for argument in arguments {
                    argument.collect_nodes(span, &mut children)?;
                }
                output.push(call(name, children, span));
            }
            Self::Variable { name } => {
                validate_type_name(name, "variable")?;
                output.push(atom(name.clone(), span));
            }
            Self::Owned { inner } => collect_prefixed("Owned", inner, span, output)?,
            Self::Ref { inner } => collect_prefixed("Ref", inner, span, output)?,
            Self::RefMut { inner } => collect_prefixed("RefMut", inner, span, output)?,
            Self::List { element } => collect_prefixed("List", element, span, output)?,
            Self::Option { value } => collect_prefixed("Option", value, span, output)?,
            Self::Result { ok, error } => {
                output.push(atom("Result".into(), span));
                ok.collect_nodes(span, output)?;
                error.collect_nodes(span, output)?;
            }
        }
        Ok(())
    }

    pub(crate) fn measure(&self, depth: u32, counts: &mut super::ExpressionCounts) {
        counts.nodes = counts.nodes.saturating_add(1);
        counts.depth = counts.depth.max(depth);
        match self {
            Self::Product { name }
            | Self::Variable { name }
            | Self::Capability { capability: name } => {
                counts.string_bytes = counts.string_bytes.saturating_add(name.len() as u64);
            }
            Self::Enum { name, arguments } => {
                counts.string_bytes = counts.string_bytes.saturating_add(name.len() as u64);
                for argument in arguments {
                    argument.measure(depth.saturating_add(1), counts);
                }
            }
            Self::Owned { inner }
            | Self::Ref { inner }
            | Self::RefMut { inner }
            | Self::List { element: inner }
            | Self::Option { value: inner } => inner.measure(depth.saturating_add(1), counts),
            Self::Result { ok, error } => {
                ok.measure(depth.saturating_add(1), counts);
                error.measure(depth.saturating_add(1), counts);
            }
            _ => {}
        }
    }
}

fn collect_prefixed(
    prefix: &str,
    inner: &TypeExpression,
    span: SourceSpan,
    output: &mut Vec<SourceNode>,
) -> Result<(), String> {
    output.push(atom(prefix.into(), span));
    inner.collect_nodes(span, output)
}

fn call(name: &str, children: Vec<SourceNode>, span: SourceSpan) -> SourceNode {
    SourceNode {
        kind: SyntaxKind::Call {
            name: name.to_string(),
        },
        span,
        leading_trivia: Vec::new(),
        before_close_trivia: Vec::new(),
        children,
    }
}

fn validate_type_name(name: &str, context: &str) -> Result<(), String> {
    if crate::source::is_source_identifier(name)
        && name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(format!("invalid {context} type name {name:?}"))
    }
}

fn atom(name: String, span: SourceSpan) -> SourceNode {
    SourceNode {
        kind: SyntaxKind::Symbol { name },
        span,
        leading_trivia: Vec::new(),
        before_close_trivia: Vec::new(),
        children: Vec::new(),
    }
}
