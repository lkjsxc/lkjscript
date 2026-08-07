use serde::{Deserialize, Serialize};

use crate::source::{SourceNode, SourceSpan, SyntaxKind};

#[derive(Debug, Serialize)]
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

#[derive(Deserialize)]
#[serde(
    remote = "TypeExpression",
    tag = "kind",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
enum TypeExpressionDef {
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

impl<'de> Deserialize<'de> for TypeExpression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::stack::grow(|| TypeExpressionDef::deserialize(deserializer))
    }
}

impl Clone for TypeExpression {
    fn clone(&self) -> Self {
        crate::stack::grow(|| match self {
            Self::Never {} => Self::Never {},
            Self::Unit {} => Self::Unit {},
            Self::Bool {} => Self::Bool {},
            Self::I64 {} => Self::I64 {},
            Self::F64 {} => Self::F64 {},
            Self::String {} => Self::String {},
            Self::Bytes {} => Self::Bytes {},
            Self::ByteVector {} => Self::ByteVector {},
            Self::ByteSlice {} => Self::ByteSlice {},
            Self::ByteSliceMut {} => Self::ByteSliceMut {},
            Self::Path {} => Self::Path {},
            Self::Capability { capability } => Self::Capability {
                capability: capability.clone(),
            },
            Self::Symbol {} => Self::Symbol {},
            Self::Resource { resource } => Self::Resource {
                resource: resource.clone(),
            },
            Self::Product { name } => Self::Product { name: name.clone() },
            Self::Enum { name, arguments } => Self::Enum {
                name: name.clone(),
                arguments: arguments.clone(),
            },
            Self::Variable { name } => Self::Variable { name: name.clone() },
            Self::List { element } => Self::List {
                element: element.clone(),
            },
            Self::Option { value } => Self::Option {
                value: value.clone(),
            },
            Self::Result { ok, error } => Self::Result {
                ok: ok.clone(),
                error: error.clone(),
            },
        })
    }
}

impl Drop for TypeExpression {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        take_children(self, &mut pending);
        while let Some(mut expression) = pending.pop() {
            take_children(&mut expression, &mut pending);
        }
    }
}

fn take_children(expression: &mut TypeExpression, pending: &mut Vec<TypeExpression>) {
    match expression {
        TypeExpression::Enum { arguments, .. } => pending.append(arguments),
        TypeExpression::List { element } | TypeExpression::Option { value: element } => {
            pending.push(std::mem::replace(element.as_mut(), TypeExpression::Unit {}));
        }
        TypeExpression::Result { ok, error } => {
            pending.push(std::mem::replace(ok.as_mut(), TypeExpression::Unit {}));
            pending.push(std::mem::replace(error.as_mut(), TypeExpression::Unit {}));
        }
        _ => {}
    }
}

// Measurements traverse a materialized host tree; checked u64 arithmetic documents the invariant.
#[allow(clippy::expect_used)]
impl TypeExpression {
    pub(crate) fn to_atoms(&self, span: SourceSpan) -> Result<Vec<SourceNode>, String> {
        let mut output = Vec::new();
        self.collect_nodes(span, &mut output)?;
        Ok(output)
    }

    fn collect_nodes(&self, span: SourceSpan, output: &mut Vec<SourceNode>) -> Result<(), String> {
        crate::stack::grow(|| self.collect_nodes_inner(span, output))
    }

    fn collect_nodes_inner(
        &self,
        span: SourceSpan,
        output: &mut Vec<SourceNode>,
    ) -> Result<(), String> {
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

    pub(crate) fn measure(&self, depth: u64, counts: &mut super::ExpressionCounts) {
        crate::stack::grow(|| self.measure_inner(depth, counts));
    }

    fn measure_inner(&self, depth: u64, counts: &mut super::ExpressionCounts) {
        counts.nodes = counts
            .nodes
            .checked_add(1)
            .expect("host-addressable type expressions fit u64");
        counts.depth = counts.depth.max(depth);
        let next = depth
            .checked_add(1)
            .expect("host-addressable type-expression depth fits u64");
        match self {
            Self::Product { name }
            | Self::Variable { name }
            | Self::Capability { capability: name } => add_measured_string(counts, name),
            Self::Enum { name, arguments } => {
                add_measured_string(counts, name);
                for argument in arguments {
                    argument.measure(next, counts);
                }
            }
            Self::List { element: inner } | Self::Option { value: inner } => {
                inner.measure(next, counts)
            }
            Self::Result { ok, error } => {
                ok.measure(next, counts);
                error.measure(next, counts);
            }
            _ => {}
        }
    }
}

#[allow(clippy::expect_used)]
fn add_measured_string(counts: &mut super::ExpressionCounts, value: &str) {
    counts.string_bytes = counts
        .string_bytes
        .checked_add(u64::try_from(value.len()).expect("host string bytes fit u64"))
        .expect("materialized type-expression strings fit u64");
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
