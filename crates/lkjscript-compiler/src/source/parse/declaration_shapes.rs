use crate::source::{SourceNode, SyntaxKind};

pub(super) fn valid_function(children: &[SourceNode]) -> bool {
    let mut index = 0;
    if matches!(
        children.get(index),
        Some(SourceNode {
            kind: SyntaxKind::Call { name },
            children,
            ..
        }) if name == "forall" && !children.is_empty()
    ) {
        index += 1;
    }
    if matches!(
        children.get(index),
        Some(SourceNode {
            kind: SyntaxKind::Call { name },
            children,
            ..
        }) if name == "bounds" && !children.is_empty() && children.iter().all(valid_bound)
    ) {
        index += 1;
    }
    matches!(
        children.get(index..),
        Some([
            SourceNode {
                kind: SyntaxKind::Call { name: signature },
                children: signature_children,
                ..
            },
            SourceNode {
                kind: SyntaxKind::Call { name: parameters },
                ..
            },
            _
        ]) if signature == "sig"
            && parameters == "params"
            && signature_children.iter().any(is_return_arrow)
    )
}

pub(super) fn valid_product_field(field: &SourceNode) -> bool {
    matches!(
        field,
        SourceNode {
            kind: SyntaxKind::Call { name },
            children,
            ..
        } if name == "field"
            && matches!(children.as_slice(), [name_node, type_node]
                if named_form(name_node)
                    && matches!(type_node,
                        SourceNode {
                            kind: SyntaxKind::Call { name },
                            children,
                            ..
                        } if name == "type" && !children.is_empty()))
    )
}

fn valid_bound(node: &SourceNode) -> bool {
    matches!(
        node,
        SourceNode {
            kind: SyntaxKind::Call { name },
            children,
            ..
        } if name == "bound" && children.len() == 2
    )
}

fn is_return_arrow(node: &SourceNode) -> bool {
    matches!(&node.kind, SyntaxKind::Symbol { name } if name == "->")
}

fn named_form(node: &SourceNode) -> bool {
    matches!(
        node,
        SourceNode {
            kind: SyntaxKind::Call { name },
            children,
            ..
        } if name == "name" && valid_name(children)
    )
}

pub(super) fn valid_name(children: &[SourceNode]) -> bool {
    matches!(
        children,
        [SourceNode {
            kind: SyntaxKind::Str { value },
            ..
        }] if !value.is_empty()
    )
}
