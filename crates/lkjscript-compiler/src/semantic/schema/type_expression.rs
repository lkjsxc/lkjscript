use serde::{Deserialize, Serialize};

use crate::source::{SourceNode, SourceSpan, SyntaxKind};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TypeExpression {
    Unit {},
    Bool {},
    I64 {},
    F64 {},
    String {},
    Buffer {},
    Symbol {},
    Handle {},
    Product {
        name: String,
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
        let mut names = Vec::new();
        self.collect_atoms(&mut names)?;
        Ok(names.into_iter().map(|name| atom(name, span)).collect())
    }

    fn collect_atoms(&self, output: &mut Vec<String>) -> Result<(), String> {
        match self {
            Self::Unit {} => output.push("Unit".into()),
            Self::Bool {} => output.push("Bool".into()),
            Self::I64 {} => output.push("I64".into()),
            Self::F64 {} => output.push("F64".into()),
            Self::String {} => output.push("Str".into()),
            Self::Buffer {} => output.push("Buf".into()),
            Self::Symbol {} => output.push("Symbol".into()),
            Self::Handle {} => output.push("Handle".into()),
            Self::Product { name } => {
                validate_type_name(name, "product")?;
                output.push("Product".into());
                output.push(name.clone());
            }
            Self::Variable { name } => {
                validate_type_name(name, "variable")?;
                output.push(name.clone());
            }
            Self::Owned { inner } => collect_prefixed("Owned", inner, output)?,
            Self::Ref { inner } => collect_prefixed("Ref", inner, output)?,
            Self::RefMut { inner } => collect_prefixed("RefMut", inner, output)?,
            Self::List { element } => collect_prefixed("List", element, output)?,
            Self::Option { value } => collect_prefixed("Option", value, output)?,
            Self::Result { ok, error } => {
                output.push("Result".into());
                ok.collect_atoms(output)?;
                error.collect_atoms(output)?;
            }
        }
        Ok(())
    }

    pub(crate) fn measure(&self, depth: u32, counts: &mut super::ExpressionCounts) {
        counts.nodes = counts.nodes.saturating_add(1);
        counts.depth = counts.depth.max(depth);
        match self {
            Self::Product { name } | Self::Variable { name } => {
                counts.string_bytes = counts.string_bytes.saturating_add(name.len() as u64);
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
    output: &mut Vec<String>,
) -> Result<(), String> {
    output.push(prefix.into());
    inner.collect_atoms(output)
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
