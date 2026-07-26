use crate::hir::Type;
use crate::source::SyntaxKind;

use super::super::site::HoleSite;

pub(super) fn nearest_loop_type(site: &HoleSite<'_>) -> Option<Type> {
    let mut node = site.root;
    let mut result = None;
    for index in &site.path {
        if let SyntaxKind::Call { name } = &node.kind {
            if name == "while" {
                result = Some(Type::Unit);
            } else if name == "loop" {
                result = node
                    .children
                    .first()
                    .and_then(super::super::types::type_form);
            }
        }
        node = node.children.get(*index)?;
    }
    result
}

pub(super) fn loop_depth(site: &HoleSite<'_>) -> u32 {
    let mut node = site.root;
    let mut depth = 0_u32;
    for index in &site.path {
        if matches!(&node.kind, SyntaxKind::Call { name } if name == "while" || name == "loop") {
            depth = depth.saturating_add(1);
        }
        let Some(child) = node.children.get(*index) else {
            break;
        };
        node = child;
    }
    depth
}
