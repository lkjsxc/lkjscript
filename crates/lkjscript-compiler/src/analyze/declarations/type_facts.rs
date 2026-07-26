mod parsing;

use crate::analyze::*;

pub(in crate::analyze) use parsing::*;

pub(in crate::analyze) fn collect_type_params<'a>(ty: &'a Type, output: &mut HashSet<&'a str>) {
    match ty {
        Type::Param(parameter) => {
            output.insert(parameter);
        }
        Type::Owned(inner)
        | Type::Ref(inner)
        | Type::RefMut(inner)
        | Type::List(inner)
        | Type::Option(inner) => collect_type_params(inner, output),
        Type::Enum { arguments, .. } => {
            for argument in arguments {
                collect_type_params(argument, output);
            }
        }
        Type::Result(ok, error) => {
            collect_type_params(ok, output);
            collect_type_params(error, output);
        }
        Type::Fn { params, ret } => {
            for parameter in params {
                collect_type_params(parameter, output);
            }
            collect_type_params(ret, output);
        }
        Type::Forall { body, .. } => collect_type_params(body, output),
        _ => {}
    }
}

pub(in crate::analyze) fn contains_ownership_type(ty: &Type) -> bool {
    match ty {
        Type::Owned(_) | Type::Ref(_) | Type::RefMut(_) => true,
        Type::List(inner) | Type::Option(inner) => contains_ownership_type(inner),
        Type::Enum { arguments, .. } => arguments.iter().any(contains_ownership_type),
        Type::Result(ok, error) => contains_ownership_type(ok) || contains_ownership_type(error),
        Type::Fn { params, ret } => {
            params.iter().any(contains_ownership_type) || contains_ownership_type(ret)
        }
        Type::Forall { body, .. } => contains_ownership_type(body),
        _ => false,
    }
}

pub(in crate::analyze) fn contains_reference_type(ty: &Type) -> bool {
    match ty {
        Type::Ref(_) | Type::RefMut(_) => true,
        Type::Owned(inner) | Type::List(inner) | Type::Option(inner) => {
            contains_reference_type(inner)
        }
        Type::Enum { arguments, .. } => arguments.iter().any(contains_reference_type),
        Type::Result(ok, error) => contains_reference_type(ok) || contains_reference_type(error),
        Type::Fn { params, ret } => {
            params.iter().any(contains_reference_type) || contains_reference_type(ret)
        }
        Type::Forall { body, .. } => contains_reference_type(body),
        _ => false,
    }
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
