use crate::analyze::*;

pub(in crate::analyze) fn parse_signature(
    args: &[AstExpr],
) -> std::result::Result<(Vec<Type>, Type), String> {
    let [inputs, output] = args else {
        return Err("sig requires exactly inputs/ and output/ fields".into());
    };
    let inputs = match inputs {
        AstExpr::Call { name, args } if name == "inputs" => {
            args.iter()
                .map(parameter_type)
                .collect::<std::result::Result<Vec<_>, _>>()?
        }
        _ => return Err("sig requires inputs/ first".into()),
    };
    let output = match output {
        AstExpr::Call { name, args } if name == "output" => match args.as_slice() {
            [ty] => parameter_type(ty)?,
            _ => return Err("output requires exactly one type".into()),
        },
        _ => return Err("sig requires output/ second".into()),
    };
    Ok((inputs, output))
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
            AstExpr::LitUnit => atoms.push("unit".into()),
            _ => return Err("type atoms must be canonical names".into()),
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
        AstExpr::LitUnit => Ok(Type::Unit),
        AstExpr::Symbol(name) => atom_type(name),
        AstExpr::Call { name, args } if args.is_empty() && is_builtin_type_name(name) => {
            atom_type(name)
        }
        AstExpr::Call { name, args } if name == "capability" && args.len() == 1 => {
            let kind = symbolic_name(&args[0])?;
            lkjscript_core::CapabilityKind::parse(&kind)
                .map(Type::Capability)
                .ok_or_else(|| format!("unknown capability kind {kind}"))
        }
        AstExpr::Call { name, args } if name == "list" && args.len() == 1 => {
            Ok(Type::List(Box::new(parameter_type(&args[0])?)))
        }
        AstExpr::Call { name, args } if name == "option" && args.len() == 1 => {
            Ok(crate::types::option_type(parameter_type(&args[0])?))
        }
        AstExpr::Call { name, args } if name == "result" && args.len() == 2 => Ok(
            crate::types::result_type(parameter_type(&args[0])?, parameter_type(&args[1])?),
        ),
        AstExpr::Call { name, args } if name == "product" && args.len() == 1 => {
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
