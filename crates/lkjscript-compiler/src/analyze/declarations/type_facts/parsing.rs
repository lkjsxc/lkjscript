use crate::analyze::*;

pub(in crate::analyze) fn parse_signature(
    args: &[AstExpr],
) -> std::result::Result<(Vec<Type>, Type), String> {
    let arrow = args
        .iter()
        .position(|argument| matches!(argument, AstExpr::Symbol(name) if name == "->"))
        .ok_or_else(|| "sig requires -> before return type".to_string())?;
    let mut parameters = Vec::new();
    let mut index = 0;
    while index < arrow {
        let (ty, next) = parse_signature_type(args, index, arrow)?;
        parameters.push(ty);
        index = next;
    }
    let (return_type, end) = parse_signature_type(args, arrow + 1, args.len())?;
    if end != args.len() {
        return Err("sig requires exactly one return type after ->".into());
    }
    Ok((parameters, return_type))
}

fn parse_signature_type(
    args: &[AstExpr],
    index: usize,
    end: usize,
) -> std::result::Result<(Type, usize), String> {
    let expression = args
        .get(index)
        .ok_or_else(|| "sig missing type".to_string())?;
    if matches!(
        expression,
        AstExpr::Call { name, args }
            if !args.is_empty()
                || (is_declaration_type_name(name) && !is_builtin_type_name(name))
    ) {
        return parameter_type(expression).map(|ty| (ty, index + 1));
    }
    let mut atoms = Vec::new();
    for expression in &args[index..end] {
        match expression {
            AstExpr::Symbol(atom) => atoms.push(atom.clone()),
            AstExpr::Call { name, args } if args.is_empty() => atoms.push(name.clone()),
            AstExpr::Call { .. } => break,
            _ => return Err("invalid type expression in sig".into()),
        }
    }
    let (ty, used) = parse_one(&atoms, 0)?;
    Ok((ty, index + used))
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
        AstExpr::Call { name, args } if name == "Capability" && args.len() == 1 => {
            let kind = symbolic_name(&args[0])?;
            lkjscript_core::CapabilityKind::parse(&kind)
                .map(Type::Capability)
                .ok_or_else(|| format!("unknown capability kind {kind}"))
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
            Ok(crate::types::option_type(parameter_type(&args[0])?))
        }
        AstExpr::Call { name, args } if name == "Result" && args.len() == 2 => Ok(
            crate::types::result_type(parameter_type(&args[0])?, parameter_type(&args[1])?),
        ),
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
