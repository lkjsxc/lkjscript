use crate::analyze::*;

pub(in crate::analyze) fn parse_signature(
    args: &[AstExpr],
) -> std::result::Result<(Vec<Type>, Type), String> {
    let nested = args.iter().any(|argument| match argument {
        AstExpr::Call { name, args } => {
            !args.is_empty() || (is_declaration_type_name(name) && !is_builtin_type_name(name))
        }
        _ => false,
    });
    if nested {
        let arrow = args
            .iter()
            .position(|argument| matches!(argument, AstExpr::Symbol(name) if name == "->"))
            .ok_or_else(|| "sig requires -> before return type".to_string())?;
        let [return_expression] = args.get(arrow + 1..).unwrap_or_default() else {
            return Err("sig requires exactly one return type after ->".into());
        };
        let parameters = args[..arrow]
            .iter()
            .map(parameter_type)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok((parameters, parameter_type(return_expression)?))
    } else {
        Type::parse_atoms(&type_atoms(args)?)
    }
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

fn type_atoms(args: &[AstExpr]) -> std::result::Result<Vec<String>, String> {
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
        AstExpr::Call { name, args } if args.is_empty() && is_builtin_type_name(name) => {
            atom_type(name)
        }
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
        AstExpr::Call { name, args } if is_declaration_type_name(name) => Ok(Type::Enum {
            id: EnumId::UNRESOLVED,
            name: name.clone(),
            arguments: args
                .iter()
                .map(parameter_type)
                .collect::<std::result::Result<Vec<_>, _>>()?,
        }),
        _ => Err("invalid parameter type expression".into()),
    }
}

fn atom_type(name: &str) -> std::result::Result<Type, String> {
    let (ty, end) = parse_one(&[name.to_string()], 0)?;
    if end == 1 {
        Ok(ty)
    } else {
        Err(format!("bad type {name}"))
    }
}
