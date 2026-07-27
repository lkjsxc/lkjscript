use crate::source::{DeclarationKind, SourceNode, SyntaxKind};

pub(super) fn rename_references(
    node: &mut SourceNode,
    kind: DeclarationKind,
    old: &str,
    new: &str,
    expression: bool,
    shadowed: bool,
) {
    match kind {
        DeclarationKind::Product => rename_product_references(node, old, new),
        DeclarationKind::Trait => rename_trait_references(node, old, new),
        DeclarationKind::Function => {
            rename_function_references(node, old, new, expression, shadowed);
            return;
        }
        DeclarationKind::Main | DeclarationKind::Enum | DeclarationKind::Implementation => {}
    }
    for child in &mut node.children {
        rename_references(child, kind, old, new, false, false);
    }
}

fn rename_function_references(
    node: &mut SourceNode,
    old: &str,
    new: &str,
    expression: bool,
    shadowed: bool,
) {
    if expression && !shadowed {
        match &mut node.kind {
            SyntaxKind::Call { name } | SyntaxKind::Symbol { name } if name == old => {
                *name = new.to_string()
            }
            _ => {}
        }
    }
    let SyntaxKind::Call { name } = &node.kind else {
        return;
    };
    match (expression, name.as_str()) {
        (false, "main") => rename_child(node, 1, old, new, false),
        (false, "fn") => rename_function_body(node, old, new),
        (false, _) => {
            for child in &mut node.children {
                rename_function_references(child, old, new, false, false);
            }
        }
        (true, "var") => rename_var(node, old, new, shadowed),
        (true, "let") => rename_let(node, old, new, shadowed),
        (true, "set") => rename_child(node, 1, old, new, shadowed),
        (true, "quote" | "move" | "borrow" | "borrow-mut") => {}
        (true, "product-value") => {
            for field in node.children.iter_mut().skip(1) {
                if let Some(value) = field.children.get_mut(1) {
                    rename_function_references(value, old, new, true, shadowed);
                }
            }
        }
        (true, "field") => rename_child(node, 0, old, new, shadowed),
        (true, "with-field") => {
            rename_child(node, 0, old, new, shadowed);
            rename_child(node, 2, old, new, shadowed);
        }
        (true, _) => {
            for child in &mut node.children {
                rename_function_references(child, old, new, true, shadowed);
            }
        }
    }
}

fn rename_function_body(node: &mut SourceNode, old: &str, new: &str) {
    let parameter_shadow = node
        .children
        .iter()
        .find(|child| matches!(&child.kind, SyntaxKind::Call { name } if name == "params"))
        .is_some_and(|params| {
            params
                .children
                .iter()
                .step_by(2)
                .any(|parameter| symbolic_name(parameter) == Some(old))
        });
    if let Some(body) = node.children.last_mut() {
        rename_function_references(body, old, new, true, parameter_shadow);
    }
}

fn rename_var(node: &mut SourceNode, old: &str, new: &str, shadowed: bool) {
    rename_child(node, 2, old, new, shadowed);
    let binding_shadow = node
        .children
        .first()
        .and_then(declared_name)
        .is_some_and(|name| name == old);
    rename_child(node, 3, old, new, shadowed || binding_shadow);
}

fn rename_let(node: &mut SourceNode, old: &str, new: &str, mut shadowed: bool) {
    let Some(body_index) = node.children.len().checked_sub(1) else {
        return;
    };
    for binding in &mut node.children[..body_index] {
        if let Some(value) = binding.children.last_mut() {
            rename_function_references(value, old, new, true, shadowed);
        }
        shadowed |= binding
            .children
            .first()
            .and_then(symbolic_name)
            .is_some_and(|name| name == old);
    }
    rename_child(node, body_index, old, new, shadowed);
}

fn rename_child(node: &mut SourceNode, index: usize, old: &str, new: &str, shadowed: bool) {
    if let Some(child) = node.children.get_mut(index) {
        rename_function_references(child, old, new, true, shadowed);
    }
}

fn declared_name(node: &SourceNode) -> Option<&str> {
    if !matches!(&node.kind, SyntaxKind::Call { name } if name == "name") {
        return None;
    }
    node.children.first().and_then(symbolic_name)
}

fn symbolic_name(node: &SourceNode) -> Option<&str> {
    match &node.kind {
        SyntaxKind::Str { value } => Some(value),
        SyntaxKind::Symbol { name } => Some(name),
        _ => None,
    }
}

fn rename_product_references(node: &mut SourceNode, old: &str, new: &str) {
    if matches!(
        &node.kind,
        SyntaxKind::Call { name } if matches!(name.as_str(), "sig" | "params" | "type" | "for")
    ) {
        for index in 0..node.children.len().saturating_sub(1) {
            let (left, right) = node.children.split_at_mut(index + 1);
            if matches!(&left[index].kind, SyntaxKind::Symbol { name } if name == "product") {
                if let SyntaxKind::Symbol { name } = &mut right[0].kind {
                    if name == old {
                        *name = new.to_string();
                    }
                }
            }
        }
    }
    if matches!(&node.kind, SyntaxKind::Call { name } if name == "product-value") {
        if let Some(SourceNode {
            kind: SyntaxKind::Symbol { name },
            ..
        }) = node.children.first_mut()
        {
            if name == old {
                *name = new.to_string();
            }
        }
    }
}

fn rename_trait_references(node: &mut SourceNode, old: &str, new: &str) {
    if matches!(&node.kind, SyntaxKind::Call { name } if name == "bound" || name == "trait") {
        for child in &mut node.children {
            if let SyntaxKind::Symbol { name } = &mut child.kind {
                if name == old {
                    *name = new.to_string();
                }
            }
        }
    }
}
