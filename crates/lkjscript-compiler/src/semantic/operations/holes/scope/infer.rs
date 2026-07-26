use std::collections::BTreeMap;

use crate::hir::Type;
use crate::semantic::schema::ScopeEntity;
use crate::source::{SourceNode, SyntaxKind};

pub(super) fn expression(node: &SourceNode, scope: &BTreeMap<String, ScopeEntity>) -> Option<Type> {
    match &node.kind {
        SyntaxKind::Unit => Some(Type::Unit),
        SyntaxKind::Bool { .. } => Some(Type::Bool),
        SyntaxKind::I64 { .. } => Some(Type::I64),
        SyntaxKind::F64 { .. } => Some(Type::F64),
        SyntaxKind::Str { .. } => Some(Type::Str),
        SyntaxKind::Symbol { name } => scope
            .values()
            .find(|entry| entry.name == *name)
            .and_then(|entry| super::parse_canonical(&entry.instantiated_type)),
        SyntaxKind::Call { name } => call_type(name, scope),
    }
}

fn call_type(name: &str, scope: &BTreeMap<String, ScopeEntity>) -> Option<Type> {
    if let Some(operation) = crate::hir::Operation::from_name(name) {
        if let Type::Fn { ret, .. } = operation.signature() {
            return Some(*ret);
        }
    }
    scope
        .values()
        .find(|entry| entry.name == name)
        .and_then(|entry| super::parse_canonical(&entry.instantiated_type))
        .and_then(|ty| match ty {
            Type::Fn { ret, .. } => Some(*ret),
            _ => None,
        })
}
