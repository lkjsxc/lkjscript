use lkjscript_core::Limits;

use crate::source::{SourceNode, SourceOrigin, SourceResult, SourceSpan, SyntaxKind};

use super::declaration_shapes::{
    valid_enum, valid_function, valid_name, valid_product_field, valid_signature,
};

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
                    "def" | "main" | "imports" | "product" | "enum" | "trait" | "impl"
                ) =>
            {
                validate_declaration_shape(form, name, origin)?
            }
            _ => {
                return Err(super::limits::syntax_error(
                    origin,
                    form.span,
                    "top-level must be def, main, imports, product, enum, trait, or impl; top-level do was removed",
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
        "imports" => !form.children.is_empty() && form.children.iter().all(valid_import),
        "main" => {
            matches!(
                form.children.as_slice(),
                [SourceNode {
                    kind: SyntaxKind::Call { name },
                    children,
                    ..
                }, _] if name == "sig" && valid_signature(children)
            ) || matches!(
                form.children.as_slice(),
                [SourceNode {
                    kind: SyntaxKind::Call { name },
                    children,
                    ..
                }, SourceNode { kind: SyntaxKind::Call { name: params }, .. }, _]
                    if name == "sig"
                        && params == "params"
                        && valid_signature(children)
            )
        }
        "def" => valid_named_body(&form.children, "fn", valid_function),
        "product" => valid_named_body(&form.children, "fields", |fields| {
            fields.iter().all(valid_product_field)
        }),
        "enum" => valid_enum(&form.children),
        "trait" => valid_named_only(&form.children),
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

fn valid_named_only(children: &[SourceNode]) -> bool {
    matches!(
        children,
        [SourceNode {
            kind: SyntaxKind::Call { name },
            children,
            ..
        }] if name == "name" && valid_name(children)
    ) || matches!(
        children,
        [SourceNode {
            kind: SyntaxKind::Call { name },
            children,
            ..
        }, SourceNode { kind: SyntaxKind::Symbol { name: visibility }, .. }]
            if name == "name" && valid_name(children) && visibility == "public"
    )
}

fn valid_named_body(
    children: &[SourceNode],
    body_name: &str,
    valid_body: impl Fn(&[SourceNode]) -> bool,
) -> bool {
    let (name, body) = match children {
        [name, body] => (name, body),
        [name, SourceNode {
            kind: SyntaxKind::Symbol { name: visibility },
            ..
        }, body]
            if visibility == "public" =>
        {
            (name, body)
        }
        _ => return false,
    };
    matches!(&name.kind, SyntaxKind::Call { name } if name == "name")
        && valid_name(&name.children)
        && matches!(&body.kind, SyntaxKind::Call { name } if name == body_name)
        && valid_body(&body.children)
}

fn valid_import(node: &SourceNode) -> bool {
    let SourceNode {
        kind: SyntaxKind::Call { name },
        children,
        ..
    } = node
    else {
        return false;
    };
    let [module, declarations] = children.as_slice() else {
        return false;
    };
    let module = matches!(module,
        SourceNode { kind: SyntaxKind::Call { name }, children, .. }
            if name == "module" && matches!(children.as_slice(),
                [SourceNode { kind: SyntaxKind::Str { value }, .. }] if valid_module(value)));
    let names = match declarations {
        SourceNode {
            kind: SyntaxKind::Call { name },
            children,
            ..
        } if name == "declarations" && !children.is_empty() => children,
        _ => return false,
    };
    name == "import"
        && module
        && names.iter().all(|node| {
            matches!(node,
            SourceNode { kind: SyntaxKind::Symbol { name }, .. }
                if super::names::is_source_identifier(name))
        })
        && names.windows(2).all(|pair| match pair {
            [SourceNode {
                kind: SyntaxKind::Symbol { name: left },
                ..
            }, SourceNode {
                kind: SyntaxKind::Symbol { name: right },
                ..
            }] => left < right,
            _ => false,
        })
}

fn valid_module(path: &str) -> bool {
    !path.is_empty()
        && path.ends_with(".lkjscript")
        && !path.starts_with('.')
        && !path.contains('#')
}
