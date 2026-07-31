use serde::{Deserialize, Serialize};

use crate::source::{SourceNode, SourceSpan, SyntaxKind};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum TypeExpression {
    Never {},
    Unit {},
    Bool {},
    I64 {},
    F64 {},
    String {},
    Bytes {},
    ByteVector {},
    ByteSlice {},
    ByteSliceMut {},
    Path {},
    Capability {
        capability: String,
    },
    Symbol {},
    Resource {
        resource: String,
    },
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
            Self::Never {} => output.push(atom("never".into(), span)),
            Self::Unit {} => output.push(atom("unit".into(), span)),
            Self::Bool {} => output.push(atom("bool".into(), span)),
            Self::I64 {} => output.push(atom("i64".into(), span)),
            Self::F64 {} => output.push(atom("f64".into(), span)),
            Self::String {} => output.push(atom("string".into(), span)),
            Self::Bytes {} => output.push(atom("bytes".into(), span)),
            Self::ByteVector {} => output.push(atom("byte-vector".into(), span)),
            Self::ByteSlice {} => output.push(atom("byte-slice".into(), span)),
            Self::ByteSliceMut {} => output.push(atom("byte-slice-mut".into(), span)),
            Self::Path {} => output.push(atom("path".into(), span)),
            Self::Capability { capability } => {
                if lkjscript_core::CapabilityKind::parse(capability).is_none() {
                    return Err(format!("unknown capability kind {capability}"));
                }
                output.push(call(
                    "capability",
                    vec![atom(capability.clone(), span)],
                    span,
                ));
            }
            Self::Symbol {} => output.push(atom("symbol".into(), span)),
            Self::Resource { resource } => {
                if lkjscript_core::ResourceKind::parse(resource).is_none() {
                    return Err(format!("unknown resource kind {resource}"));
                }
                output.push(atom(resource.clone(), span));
            }
            Self::Product { name } => {
                validate_type_name(name, "product")?;
                output.push(call("product", vec![atom(name.clone(), span)], span));
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
            Self::List { element } => collect_prefixed("list", element, span, output)?,
            Self::Option { value } => collect_prefixed("option", value, span, output)?,
            Self::Result { ok, error } => {
                let mut children = Vec::new();
                ok.collect_nodes(span, &mut children)?;
                error.collect_nodes(span, &mut children)?;
                output.push(call("result", children, span));
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
            Self::List { element: inner } | Self::Option { value: inner } => {
                inner.measure(depth.saturating_add(1), counts)
            }
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
    let mut children = Vec::new();
    inner.collect_nodes(span, &mut children)?;
    output.push(call(prefix, children, span));
    Ok(())
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
    if crate::source::is_source_identifier(name) {
        Ok(())
    } else {
        Err(format!("invalid {context} type name {name:?}"))
    }
}

fn atom(name: String, span: SourceSpan) -> SourceNode {
    SourceNode {
        kind: if name == "unit" {
            SyntaxKind::Unit
        } else {
            SyntaxKind::Symbol { name }
        },
        span,
        leading_trivia: Vec::new(),
        before_close_trivia: Vec::new(),
        children: Vec::new(),
    }
}
