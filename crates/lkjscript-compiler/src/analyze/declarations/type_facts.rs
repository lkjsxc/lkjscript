use crate::analyze::*;

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
        Type::Result(ok, error) => contains_reference_type(ok) || contains_reference_type(error),
        Type::Fn { params, ret } => {
            params.iter().any(contains_reference_type) || contains_reference_type(ret)
        }
        Type::Forall { body, .. } => contains_reference_type(body),
        _ => false,
    }
}

pub(in crate::analyze) fn parse_signature(
    args: &[AstExpr],
) -> std::result::Result<(Vec<Type>, Type), String> {
    let atoms = type_atoms(args)?;
    Type::parse_atoms(&atoms)
}

pub(in crate::analyze) fn parse_type_form(args: &[AstExpr]) -> std::result::Result<Type, String> {
    if args.len() == 1 {
        return parameter_type(&args[0]);
    }
    let atoms = type_atoms(args)?;
    let (ty, end) = parse_one(&atoms, 0)?;
    if end != atoms.len() {
        return Err("trailing tokens in type/".into());
    }
    Ok(ty)
}

pub(in crate::analyze) fn type_atoms(args: &[AstExpr]) -> std::result::Result<Vec<String>, String> {
    let mut atoms = Vec::with_capacity(args.len());
    for argument in args {
        match argument {
            AstExpr::Symbol(atom) => atoms.push(atom.clone()),
            AstExpr::Call { name, args } if args.is_empty() => atoms.push(name.clone()),
            _ => return Err("type atoms must be names or ->".into()),
        }
    }
    Ok(atoms)
}

pub(in crate::analyze) fn parse_typed_params(
    args: &[AstExpr],
) -> std::result::Result<(Vec<String>, Vec<Type>), String> {
    if !args.len().is_multiple_of(2) {
        return Err("params must be name Type pairs".into());
    }
    let mut names = Vec::with_capacity(args.len() / 2);
    let mut types = Vec::with_capacity(args.len() / 2);
    let mut index = 0;
    while index < args.len() {
        names.push(symbolic_name(&args[index])?);
        types.push(parameter_type(&args[index + 1])?);
        index += 2;
    }
    Ok((names, types))
}

pub(in crate::analyze) fn parameter_type(
    expression: &AstExpr,
) -> std::result::Result<Type, String> {
    match expression {
        AstExpr::Symbol(name) => atom_type(name),
        AstExpr::Call { name, args } if args.is_empty() => atom_type(name),
        AstExpr::Call { name, args }
            if matches!(name.as_str(), "Owned" | "Ref" | "RefMut") && args.len() == 1 =>
        {
            let inner = parameter_type(&args[0])?;
            if inner != Type::Buf {
                return Err(format!(
                    "{name} accepts only exact Buf in the initial ownership slice"
                ));
            }
            Ok(match name.as_str() {
                "Owned" => Type::Owned(Box::new(inner)),
                "Ref" => Type::Ref(Box::new(inner)),
                "RefMut" => Type::RefMut(Box::new(inner)),
                _ => return Err("invalid ownership parameter type".into()),
            })
        }
        AstExpr::Call { name, args } if name == "List" && args.len() == 1 => {
            Ok(Type::List(Box::new(parameter_type(&args[0])?)))
        }
        AstExpr::Call { name, args } if name == "Option" && args.len() == 1 => {
            Ok(Type::Option(Box::new(parameter_type(&args[0])?)))
        }
        AstExpr::Call { name, args } if name == "Result" && args.len() == 2 => Ok(Type::Result(
            Box::new(parameter_type(&args[0])?),
            Box::new(parameter_type(&args[1])?),
        )),
        AstExpr::Call { name, args } if name == "Product" && args.len() == 1 => {
            Ok(Type::Product(symbolic_name(&args[0])?))
        }
        _ => Err("invalid parameter type expression".into()),
    }
}

pub(in crate::analyze) fn atom_type(name: &str) -> std::result::Result<Type, String> {
    let (ty, end) = parse_one(&[name.to_string()], 0)?;
    if end == 1 {
        Ok(ty)
    } else {
        Err(format!("bad type {name}"))
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
