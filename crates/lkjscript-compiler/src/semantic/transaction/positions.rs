use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode};
use crate::source::{SourceNode, SyntaxKind, ValidatedSourceTree};

pub(crate) fn path_from_owner(
    tree: &ValidatedSourceTree,
    owner: u64,
    target: u64,
) -> Result<Vec<usize>, ProtocolError> {
    let mut current = target;
    let mut reversed = Vec::new();
    while current != owner {
        let current_index = usize::try_from(current).map_err(|_| {
            error(
                ProtocolErrorCode::UnknownNode,
                "node path identity is not host-addressable",
            )
        })?;
        let node = tree
            .nodes()
            .get(current_index)
            .ok_or_else(|| error(ProtocolErrorCode::UnknownNode, "node path is incomplete"))?;
        let parent = node.parent().ok_or_else(|| {
            error(
                ProtocolErrorCode::InvalidOperation,
                "node escapes its declaration",
            )
        })?;
        let parent_index = usize::try_from(parent.index()).map_err(|_| {
            error(
                ProtocolErrorCode::UnknownNode,
                "parent node identity is not host-addressable",
            )
        })?;
        let parent_node = tree
            .nodes()
            .get(parent_index)
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

fn expression_descendant(mut node: &SourceNode, mut path: &[usize]) -> bool {
    loop {
        let Some((&index, rest)) = path.split_first() else {
            return true;
        };
        let SyntaxKind::Call { name } = &node.kind else {
            return false;
        };
        match name.as_str() {
            "quote" | "move" | "borrow" | "borrow-mut" => return false,
            "product-value" => {
                let Some(field) = node.children.get(index) else {
                    return false;
                };
                if index == 0
                    || !matches!(&field.kind, SyntaxKind::Call { name } if name == "field")
                    || rest.first() != Some(&1)
                {
                    return false;
                }
                let Some(value) = field.children.get(1) else {
                    return false;
                };
                node = value;
                path = &rest[1..];
            }
            "variant-value" => {
                if index != 2 || rest.len() < 2 {
                    return false;
                }
                let Some(field) = node
                    .children
                    .get(2)
                    .and_then(|fields| fields.children.get(rest[0]))
                else {
                    return false;
                };
                if rest[1] != 1
                    || !matches!(&field.kind, SyntaxKind::Call { name } if name == "variant-field")
                {
                    return false;
                }
                let Some(value) = field.children.get(1) else {
                    return false;
                };
                node = value;
                path = &rest[2..];
            }
            "match" => {
                if index == 0 {
                    let Some(value) = node.children.first() else {
                        return false;
                    };
                    node = value;
                    path = rest;
                    continue;
                }
                if index != 1 || rest.len() < 2 {
                    return false;
                }
                let Some(arm) = node
                    .children
                    .get(1)
                    .and_then(|arms| arms.children.get(rest[0]))
                else {
                    return false;
                };
                if rest[1] != 1 || !matches!(&arm.kind, SyntaxKind::Call { name } if name == "arm")
                {
                    return false;
                }
                let Some(body) = arm.children.get(1) else {
                    return false;
                };
                node = body;
                path = &rest[2..];
            }
            "let" if index + 1 != node.children.len() => {
                let Some(binding) = node.children.get(index) else {
                    return false;
                };
                let Some(value_index) = binding.children.len().checked_sub(1) else {
                    return false;
                };
                if !matches!(&binding.kind, SyntaxKind::Call { name } if name == "bind")
                    || rest.first() != Some(&value_index)
                {
                    return false;
                }
                let Some(value) = binding.children.get(value_index) else {
                    return false;
                };
                node = value;
                path = &rest[1..];
            }
            _ => {
                let direct = match name.as_str() {
                    "var" => index >= 2,
                    "set" => index == 1,
                    "field" => index == 0,
                    "with-field" => index == 0 || index == 2,
                    _ => true,
                };
                if !direct {
                    return false;
                }
                let Some(child) = node.children.get(index) else {
                    return false;
                };
                node = child;
                path = rest;
            }
        }
    }
}
