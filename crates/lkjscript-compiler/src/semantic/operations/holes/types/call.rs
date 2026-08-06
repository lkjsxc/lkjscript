use std::collections::HashMap;

use crate::hir::Type;
use crate::source::{SourceNode, SyntaxKind, ValidatedSourceTree};

pub(crate) fn parameter_type(
    call: &SourceNode,
    name: &str,
    index: usize,
    inherited: Option<&Type>,
    tree: &ValidatedSourceTree,
) -> Option<Type> {
    let (params, result) = if let Some(operation) = crate::hir::Operation::from_name(name) {
        match &operation.signature() {
            Type::Fn { params, ret } => (params.clone(), ret.as_ref().clone()),
            _ => return None,
        }
    } else {
        super::super::scope::function_signatures(tree)
            .into_iter()
            .find(|entry| entry.0 == name)
            .map(|entry| (entry.1, entry.2))?
    };
    let mut substitutions = HashMap::new();
    if let Some(inherited) = inherited {
        bind_parameters(&result, inherited, &mut substitutions);
    }
    for (position, (parameter, argument)) in params.iter().zip(&call.children).enumerate() {
        if position == index {
            continue;
        }
        if let Some(actual) = simple_expression_type(argument) {
            bind_parameters(parameter, &actual, &mut substitutions);
        }
    }
    let expected = params.get(index)?.subst(&substitutions);
    (!contains_parameter(&expected)).then_some(expected)
}

fn bind_parameters(pattern: &Type, actual: &Type, output: &mut HashMap<String, Type>) {
    match (pattern, actual) {
        (Type::Param(name), actual) => {
            output.entry(name.clone()).or_insert_with(|| actual.clone());
        }
        (Type::List(left), Type::List(right)) => bind_parameters(left, right, output),
        (
            Type::Enum {
                id: left_id,
                arguments: left,
                ..
            },
            Type::Enum {
                id: right_id,
                arguments: right,
                ..
            },
        ) if left_id == right_id && left.len() == right.len() => {
            for (left, right) in left.iter().zip(right) {
                bind_parameters(left, right, output);
            }
        }
        _ => {}
    }
}

fn simple_expression_type(node: &SourceNode) -> Option<Type> {
    match &node.kind {
        SyntaxKind::Unit => Some(Type::Unit),
        SyntaxKind::Bool { .. } => Some(Type::Bool),
        SyntaxKind::I64 { .. } => Some(Type::I64),
        SyntaxKind::F64 { .. } => Some(Type::F64),
        SyntaxKind::Str { .. } => Some(Type::Str),
        SyntaxKind::Bytes { .. } => Some(Type::Bytes),
        SyntaxKind::Call { name } if name == "none" => {
            let (inner, used) = super::parse_type_nodes(&node.children)?;
            (used == node.children.len()).then(|| crate::types::option_type(inner))
        }
        SyntaxKind::Call { name } if name == "empty-list" => {
            let (inner, used) = super::parse_type_nodes(&node.children)?;
            (used == node.children.len()).then(|| Type::List(Box::new(inner)))
        }
        _ => None,
    }
}

fn contains_parameter(ty: &Type) -> bool {
    match ty {
        Type::Param(_) | Type::Forall { .. } => true,
        Type::List(inner) => contains_parameter(inner),
        Type::Enum { arguments, .. } => arguments.iter().any(contains_parameter),
        Type::Fn { params, ret } => {
            params.iter().any(contains_parameter) || contains_parameter(ret)
        }
        _ => false,
    }
}
