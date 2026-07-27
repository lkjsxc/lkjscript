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
            && valid_signature(signature_children)
    )
}

pub(super) fn valid_enum(children: &[SourceNode]) -> bool {
    let [name, tail @ ..] = children else {
        return false;
    };
    if !named_form(name) {
        return false;
    }
    let tail = match tail {
        [SourceNode {
            kind: SyntaxKind::Symbol { name },
            ..
        }, rest @ ..]
            if name == "public" =>
        {
            rest
        }
        rest => rest,
    };
    let variants = match tail {
        [SourceNode {
            kind: SyntaxKind::Call { name },
            children,
            ..
        }] if name == "variants" => children,
        [SourceNode {
            kind: SyntaxKind::Call { name: forall },
            children: parameters,
            ..
        }, SourceNode {
            kind: SyntaxKind::Call { name: variants },
            children,
            ..
        }] if forall == "forall"
            && variants == "variants"
            && !parameters.is_empty()
            && parameters
                .iter()
                .all(|node| matches!(node.kind, SyntaxKind::Symbol { .. })) =>
        {
            children
        }
        _ => return false,
    };
    !variants.is_empty() && variants.iter().all(valid_variant)
}

fn valid_variant(node: &SourceNode) -> bool {
    matches!(node,
        SourceNode { kind: SyntaxKind::Call { name }, children, .. }
            if name == "variant" && matches!(children.as_slice(), [variant_name, fields]
                if named_form(variant_name)
                    && matches!(fields,
                        SourceNode { kind: SyntaxKind::Call { name }, children, .. }
                            if name == "fields" && children.iter().all(valid_variant_field))))
}

fn valid_variant_field(field: &SourceNode) -> bool {
    matches!(field,
        SourceNode { kind: SyntaxKind::Call { name }, children, .. }
            if name == "variant-field" && matches!(children.as_slice(), [name_node, type_node]
                if named_form(name_node)
                    && matches!(type_node,
                        SourceNode { kind: SyntaxKind::Call { name }, children, .. }
                            if name == "type" && !children.is_empty())))
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

pub(super) fn valid_signature(children: &[SourceNode]) -> bool {
    matches!(children,
        [SourceNode { kind: SyntaxKind::Call { name: inputs }, .. },
         SourceNode { kind: SyntaxKind::Call { name: output }, children, .. }]
            if inputs == "inputs" && output == "output" && children.len() == 1)
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
        }] if super::names::is_source_identifier(value)
            && lkjscript_contracts::removed_spelling(value).is_none()
    )
}
