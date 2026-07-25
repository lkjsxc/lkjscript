use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode};
use crate::source::{SourceNode, SyntaxKind, ValidatedSourceTree};

pub(crate) fn path_from_owner(
    tree: &ValidatedSourceTree,
    owner: u32,
    target: u32,
) -> Result<Vec<usize>, ProtocolError> {
    let mut current = target;
    let mut reversed = Vec::new();
    while current != owner {
        let node = tree
            .nodes()
            .get(current as usize)
            .ok_or_else(|| error(ProtocolErrorCode::UnknownNode, "node path is incomplete"))?;
        let parent = node.parent().ok_or_else(|| {
            error(
                ProtocolErrorCode::InvalidOperation,
                "node escapes its declaration",
            )
        })?;
        let parent_node = tree
            .nodes()
            .get(parent.index() as usize)
            .ok_or_else(|| error(ProtocolErrorCode::UnknownNode, "parent node is unavailable"))?;
        let child = parent_node
            .children()
            .iter()
            .position(|id| id.index() == current)
            .ok_or_else(|| {
                error(
                    ProtocolErrorCode::UnknownNode,
                    "parent-child relation is invalid",
                )
            })?;
        reversed.push(child);
        current = parent.index();
    }
    reversed.reverse();
    Ok(reversed)
}

pub(crate) fn is_expression_path(declaration: &SourceNode, path: &[usize]) -> bool {
    let SyntaxKind::Call { name } = &declaration.kind else {
        return false;
    };
    let root_path = match name.as_str() {
        "main" => vec![1],
        "def" => {
            let Some(function) = declaration.children.get(1) else {
                return false;
            };
            vec![1, function.children.len().saturating_sub(1)]
        }
        _ => return false,
    };
    if !path.starts_with(&root_path) {
        return false;
    }
    let mut node = declaration;
    for index in &root_path {
        let Some(child) = node.children.get(*index) else {
            return false;
        };
        node = child;
    }
    expression_descendant(node, &path[root_path.len()..])
}

fn expression_descendant(node: &SourceNode, path: &[usize]) -> bool {
    let Some((&index, rest)) = path.split_first() else {
        return true;
    };
    let SyntaxKind::Call { name } = &node.kind else {
        return false;
    };
    let direct = match name.as_str() {
        "var" => index >= 2,
        "set" => index == 1,
        "quote" | "move" | "borrow" | "borrow-mut" => false,
        "product-value" => return product_field_descendant(node, index, rest),
        "field" => index == 0,
        "with-field" => index == 0 || index == 2,
        "let" if index + 1 != node.children.len() => {
            return binding_descendant(node, index, rest);
        }
        _ => true,
    };
    direct
        && node
            .children
            .get(index)
            .is_some_and(|child| expression_descendant(child, rest))
}

fn product_field_descendant(node: &SourceNode, index: usize, rest: &[usize]) -> bool {
    let Some(field) = node.children.get(index) else {
        return false;
    };
    index > 0
        && matches!(&field.kind, SyntaxKind::Call { name } if name == "field")
        && rest.first() == Some(&1)
        && field
            .children
            .get(1)
            .is_some_and(|value| expression_descendant(value, &rest[1..]))
}

fn binding_descendant(node: &SourceNode, index: usize, rest: &[usize]) -> bool {
    let Some(bind) = node.children.get(index) else {
        return false;
    };
    let Some(value_index) = bind.children.len().checked_sub(1) else {
        return false;
    };
    matches!(&bind.kind, SyntaxKind::Call { name } if name == "bind")
        && rest.first() == Some(&value_index)
        && bind
            .children
            .get(value_index)
            .is_some_and(|value| expression_descendant(value, &rest[1..]))
}
