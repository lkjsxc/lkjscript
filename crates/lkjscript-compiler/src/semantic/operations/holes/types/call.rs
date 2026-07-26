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
        match operation.signature() {
            Type::Fn { params, ret } => (params, *ret),
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
        (Type::Owned(left), Type::Owned(right))
        | (Type::Ref(left), Type::Ref(right))
        | (Type::RefMut(left), Type::RefMut(right))
        | (Type::List(left), Type::List(right))
        | (Type::Option(left), Type::Option(right)) => bind_parameters(left, right, output),
        (Type::Result(left_ok, left_error), Type::Result(right_ok, right_error)) => {
            bind_parameters(left_ok, right_ok, output);
            bind_parameters(left_error, right_error, output);
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
        SyntaxKind::Call { name } if name == "none" => {
            let (inner, used) = super::parse_type_nodes(&node.children)?;
            (used == node.children.len()).then(|| Type::Option(Box::new(inner)))
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
        Type::Owned(inner)
        | Type::Ref(inner)
        | Type::RefMut(inner)
        | Type::List(inner)
        | Type::Option(inner) => contains_parameter(inner),
        Type::Result(ok, error) => contains_parameter(ok) || contains_parameter(error),
        Type::Fn { params, ret } => {
            params.iter().any(contains_parameter) || contains_parameter(ret)
        }
        _ => false,
    }
}
