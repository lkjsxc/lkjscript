mod parsing;

use crate::analyze::*;

pub(in crate::analyze) use parsing::*;

pub(in crate::analyze) fn collect_type_params<'a>(ty: &'a Type, output: &mut HashSet<&'a str>) {
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Param(name) => {
                output.insert(name);
            }
            Type::List(inner) => pending.push(inner),
            Type::Enum { arguments, .. } => pending.extend(arguments),
            Type::Fn { params, ret } => {
                pending.push(ret);
                pending.extend(params);
            }
            Type::Forall { body, .. } => pending.push(body),
            _ => {}
        }
    }
}

pub(in crate::analyze) fn contains_ownership_type(ty: &Type) -> bool {
    visit_type(ty, |ty| {
        matches!(
            ty,
            Type::Bytes
                | Type::ByteVector
                | Type::ByteSlice
                | Type::ByteSliceMut
                | Type::Resource(_)
        )
    })
}

pub(in crate::analyze) fn contains_resource_type(ty: &Type) -> bool {
    visit_type(ty, |ty| matches!(ty, Type::Resource(_)))
}

pub(in crate::analyze) fn contains_reference_type(ty: &Type) -> bool {
    visit_type(ty, |ty| matches!(ty, Type::ByteSlice | Type::ByteSliceMut))
}

fn visit_type(ty: &Type, mut predicate: impl FnMut(&Type) -> bool) -> bool {
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        if predicate(ty) {
            return true;
        }
        match ty {
            Type::List(inner) => pending.push(inner),
            Type::Enum { arguments, .. } => pending.extend(arguments),
            Type::Fn { params, ret } => {
                pending.push(ret);
                pending.extend(params);
            }
            Type::Forall { body, .. } => pending.push(body),
            _ => {}
        }
    }
    false
}

pub(in crate::analyze) fn declared_name_form(
    expression: &AstExpr,
    context: &str,
) -> std::result::Result<String, String> {
    match expression {
        AstExpr::Call { name, args } if name == "name" => match args.as_slice() {
            [AstExpr::LitStr(name)] if !name.is_empty() => Ok(name.clone()),
            _ => Err(format!(
                "{context} name must be one non-empty name/ text line"
            )),
        },
        _ => Err(format!("{context} expects name/…/name first")),
    }
}

pub(in crate::analyze) fn symbolic_name(
    expression: &AstExpr,
) -> std::result::Result<String, String> {
    match expression {
        AstExpr::Symbol(name) => Ok(name.clone()),
        AstExpr::Call { name, args } if args.is_empty() => Ok(name.clone()),
        _ => Err("name must be a symbol".into()),
    }
}
