use lkjscript_core::Limits;

use crate::source::{SourceNode, SourceOrigin, SourceResult, SourceSpan, SyntaxKind};

pub(super) fn validate_top_level(
    forms: &[SourceNode],
    limits: &Limits,
    origin: &SourceOrigin,
) -> SourceResult<()> {
    let count = u32::try_from(forms.len()).unwrap_or(u32::MAX);
    if count > limits.max_toplevel_forms {
        return Err(super::limits::resource_error(
            origin,
            forms.first().map_or(SourceSpan::zero(), |form| form.span),
            format!(
                "too many top-level forms ({count} > {}); split via import",
                limits.max_toplevel_forms
            ),
        ));
    }
    for form in forms {
        match &form.kind {
            SyntaxKind::Call { name }
                if matches!(
                    name.as_str(),
                    "def" | "main" | "import" | "product" | "trait" | "impl"
                ) =>
            {
                validate_declaration_shape(form, name, origin)?
            }
            _ => {
                return Err(super::limits::syntax_error(
                    origin,
                    form.span,
                    "top-level must be def, main, import, product, trait, or impl; top-level do was removed",
                ));
            }
        }
    }
    Ok(())
}

fn validate_declaration_shape(
    form: &SourceNode,
    name: &str,
    origin: &SourceOrigin,
) -> SourceResult<()> {
    let valid = match name {
        "import" => matches!(
            form.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::Str { value },
                ..
            }] if !value.is_empty()
        ),
        "main" => matches!(
            form.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::Call { name },
                children,
                ..
            }, _] if name == "sig"
                && children.iter().any(|node| matches!(node.kind, SyntaxKind::Symbol { ref name } if name == "->"))
        ),
        "def" => matches!(
            form.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::Call { name: name_marker },
                children: name_children,
                ..
            }, SourceNode {
                kind: SyntaxKind::Call { name: function_marker },
                children: function_children,
                ..
            }]
                if name_marker == "name"
                    && function_marker == "fn"
                    && matches!(name_children.as_slice(), [SourceNode { kind: SyntaxKind::Str { value }, .. }] if !value.is_empty())
                    && valid_function_shape(function_children)
        ),
        "product" => matches!(
            form.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::Call { name: name_marker },
                children: name_children,
                ..
            }, SourceNode {
                kind: SyntaxKind::Call { name: fields_marker },
                children: fields,
                ..
            }]
                if name_marker == "name"
                    && fields_marker == "fields"
                    && matches!(name_children.as_slice(), [SourceNode { kind: SyntaxKind::Str { value }, .. }] if !value.is_empty())
                    && fields.iter().all(valid_product_field_shape)
        ),
        "trait" => matches!(
            form.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::Call { name: name_marker },
                children: name_children,
                ..
            }]
                if name_marker == "name"
                    && matches!(name_children.as_slice(), [SourceNode { kind: SyntaxKind::Str { value }, .. }] if !value.is_empty())
        ),
        "impl" => matches!(
            form.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::Call { name: trait_marker },
                children: trait_children,
                ..
            }, SourceNode {
                kind: SyntaxKind::Call { name: for_marker },
                children: for_children,
                ..
            }]
                if trait_marker == "trait"
                    && for_marker == "for"
                    && trait_children.len() == 1
                    && !for_children.is_empty()
        ),
        _ => false,
    };
    if valid {
        return Ok(());
    }
    Err(super::limits::syntax_error(
        origin,
        form.span,
        format!("malformed top-level {name} declaration shape"),
    ))
}

fn valid_function_shape(children: &[SourceNode]) -> bool {
    let mut index = 0;
    if matches!(children.get(index), Some(SourceNode { kind: SyntaxKind::Call { name }, children, .. }) if name == "forall" && !children.is_empty())
    {
        index += 1;
    }
    if matches!(children.get(index), Some(SourceNode { kind: SyntaxKind::Call { name }, children, .. }) if name == "bounds" && !children.is_empty() && children.iter().all(|bound| matches!(bound, SourceNode { kind: SyntaxKind::Call { name }, children, .. } if name == "bound" && children.len() == 2)))
    {
        index += 1;
    }
    matches!(
        children.get(index..),
        Some([
            SourceNode { kind: SyntaxKind::Call { name: signature }, children: signature_children, .. },
            SourceNode { kind: SyntaxKind::Call { name: parameters }, .. },
            _
        ]) if signature == "sig"
            && parameters == "params"
            && signature_children.iter().any(|node| matches!(node.kind, SyntaxKind::Symbol { ref name } if name == "->"))
    )
}

fn valid_product_field_shape(field: &SourceNode) -> bool {
    matches!(
        field,
        SourceNode {
            kind: SyntaxKind::Call { name },
            children,
            ..
        } if name == "field"
            && matches!(children.as_slice(), [
                SourceNode { kind: SyntaxKind::Call { name: name_marker }, children: name_children, .. },
                SourceNode { kind: SyntaxKind::Call { name: type_marker }, children: type_children, .. }
            ] if name_marker == "name"
                && type_marker == "type"
                && !type_children.is_empty()
                && matches!(name_children.as_slice(), [SourceNode { kind: SyntaxKind::Str { value }, .. }] if !value.is_empty()))
    )
}
