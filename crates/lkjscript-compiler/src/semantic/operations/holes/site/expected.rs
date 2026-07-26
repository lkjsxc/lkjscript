use crate::hir::Type;
use crate::semantic::schema::TypeUnavailableReason;
use crate::source::{SourceNode, SyntaxKind, ValidatedSourceTree};

pub(super) fn declaration_return(root: &SourceNode) -> Option<Type> {
    let body = if super::super::types::call_is(root, "main") {
        root
    } else if super::super::types::call_is(root, "def") {
        root.children
            .iter()
            .find(|child| super::super::types::call_is(child, "fn"))?
    } else {
        return None;
    };
    body.children
        .iter()
        .find_map(|child| super::super::types::signature(child).map(|(_, ret)| ret))
}

pub(super) fn expected_at(
    root: &SourceNode,
    path: &[usize],
    return_type: &Type,
    tree: &ValidatedSourceTree,
) -> Result<Type, TypeUnavailableReason> {
    let mut current = root;
    let mut expected = Some(return_type.clone());
    for (depth, child_index) in path.iter().copied().enumerate() {
        let target = depth + 1 == path.len();
        if target && super::super::types::call_is(current, "bind") && child_index == 1 {
            return Err(TypeUnavailableReason::UnconstrainedLetInitializer);
        }
        let parent_is_call = matches!(&current.kind, SyntaxKind::Call { name }
            if crate::hir::Operation::from_name(name).is_some()
                || super::super::scope::function_signatures(tree).iter()
                    .any(|item| item.0 == *name));
        expected = child_expectation(current, child_index, expected, tree, target);
        if target && expected.is_none() && parent_is_call {
            return Err(TypeUnavailableReason::UnsupportedBuiltinInstantiation);
        }
        current = current
            .children
            .get(child_index)
            .ok_or(TypeUnavailableReason::UnsupportedStructuralPosition)?;
    }
    expected.ok_or(TypeUnavailableReason::UnsupportedStructuralPosition)
}

fn child_expectation(
    parent: &SourceNode,
    index: usize,
    inherited: Option<Type>,
    tree: &ValidatedSourceTree,
    target: bool,
) -> Option<Type> {
    let SyntaxKind::Call { name } = &parent.kind else {
        return None;
    };
    match name.as_str() {
        "main" if index + 1 == parent.children.len() => inherited,
        "main" => None,
        "def" => inherited,
        "fn" if index + 1 == parent.children.len() => inherited,
        "fn" => None,
        "if" if index == 0 => Some(Type::Bool),
        "if" => inherited,
        "match" if index == 0 => match_scrutinee_type(parent),
        "match" => inherited,
        "arms" => inherited,
        "arm" if index == 1 => inherited,
        "arm" => None,
        "var" if index == 2 => parent
            .children
            .get(1)
            .and_then(super::super::types::type_form),
        "var" if index == 3 => inherited,
        "set" if index == 1 => None,
        "do" if index + 1 == parent.children.len() => inherited,
        "while" if index == 0 => Some(Type::Bool),
        "while" => Some(Type::Unit),
        "loop" if index == 0 => None,
        "loop" => parent
            .children
            .first()
            .and_then(super::super::types::type_form),
        "return" if index == 0 => inherited,
        "break" if index == 0 => inherited,
        "trap" if index == 0 => Some(Type::Str),
        "exit" if index == 0 => Some(Type::I64),
        "let" if index + 1 == parent.children.len() => inherited,
        "bind" if index == 1 && target => None,
        other => {
            super::super::types::call_parameter_type(parent, other, index, inherited.as_ref(), tree)
        }
    }
}

fn match_scrutinee_type(node: &SourceNode) -> Option<Type> {
    let arms = node.children.get(1)?;
    let pattern = arms.children.first()?.children.first()?;
    pattern_type(pattern)
}

fn pattern_type(pattern: &SourceNode) -> Option<Type> {
    let SyntaxKind::Call { name } = &pattern.kind else {
        return None;
    };
    match name.as_str() {
        "bool-pattern" => Some(Type::Bool),
        "i64-pattern" => Some(Type::I64),
        "variant-pattern" | "product-pattern" => pattern
            .children
            .first()
            .and_then(super::super::types::type_form),
        _ => None,
    }
}
