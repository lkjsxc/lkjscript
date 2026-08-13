use crate::analyze::*;

pub(in crate::analyze) fn parse_signature(
    analyzer: &Analyzer,
    args: &[AstExpr],
) -> std::result::Result<(Vec<Type>, Type), String> {
    let [inputs, output] = args else {
        return Err("sig requires exactly inputs/ and output/ fields".into());
    };
    let inputs = match inputs {
        AstExpr::Call { name, args } if name == "inputs" => args
            .iter()
            .map(|expression| parameter_type(analyzer, expression))
            .collect::<std::result::Result<Vec<_>, _>>()?,
        _ => return Err("sig requires inputs/ first".into()),
    };
    let output = match output {
        AstExpr::Call { name, args } if name == "output" => match args.as_slice() {
            [ty] => parameter_type(analyzer, ty)?,
            _ => return Err("output requires exactly one type".into()),
        },
        _ => return Err("sig requires output/ second".into()),
    };
    Ok((inputs, output))
}

pub(in crate::analyze) fn parse_type_form(
    analyzer: &Analyzer,
    args: &[AstExpr],
) -> std::result::Result<Type, String> {
    if args.len() == 1 {
        return parameter_type(analyzer, &args[0]);
    }
    let atoms = type_atoms(args)?;
    let (ty, end) = parse_type_atoms(analyzer, &atoms, 0)?;
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

fn parse_type_atoms(
    analyzer: &Analyzer,
    atoms: &[String],
    index: usize,
) -> std::result::Result<(Type, usize), String> {
    let Some(atom) = atoms.get(index) else {
        return Err("expected type".into());
    };
    match atom.as_str() {
        "product" => {
            let name = atoms
                .get(index + 1)
                .ok_or_else(|| "product requires a declared product name".to_owned())?;
            let id = analyzer
                .product_names
                .get(name)
                .copied()
                .ok_or_else(|| format!("unknown product type {name}"))?;
            Ok((Type::Product(id), index + 2))
        }
        "list" => {
            let (inner, next) = parse_type_atoms(analyzer, atoms, index + 1)?;
            Ok((Type::List(Box::new(inner)), next))
        }
        "option" => {
            let (inner, next) = parse_type_atoms(analyzer, atoms, index + 1)?;
            Ok((crate::types::option_type(inner), next))
        }
        "result" => {
            let (ok, next) = parse_type_atoms(analyzer, atoms, index + 1)?;
            let (error, end) = parse_type_atoms(analyzer, atoms, next)?;
            Ok((crate::types::result_type(ok, error), end))
        }
        name if analyzer.enum_headers.contains_key(name) => {
            let (id, parameters) = analyzer
                .enum_headers
                .get(name)
                .ok_or_else(|| format!("unknown enum type {name}"))?;
            let mut arguments = Vec::with_capacity(parameters.len());
            let mut next = index + 1;
            for _ in parameters {
                let (argument, end) = parse_type_atoms(analyzer, atoms, next)?;
                arguments.push(argument);
                next = end;
            }
            Ok((Type::Enum { id: *id, arguments }, next))
        }
        _ => parse_one(atoms, index),
    }
}

pub(in crate::analyze) fn parse_typed_params(
    analyzer: &Analyzer,
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
        types.push(parameter_type(analyzer, &args[index + 1])?);
        index += 2;
    }
    Ok((names, types))
}

pub(in crate::analyze) fn parameter_type(
    analyzer: &Analyzer,
    expression: &AstExpr,
) -> std::result::Result<Type, String> {
    crate::stack::grow(|| parameter_type_inner(analyzer, expression))
}

fn parameter_type_inner(
    analyzer: &Analyzer,
    expression: &AstExpr,
) -> std::result::Result<Type, String> {
    match expression {
        AstExpr::LitUnit => Ok(Type::Unit),
        AstExpr::Symbol(name) => atom_type(analyzer, name),
        AstExpr::Call { name, args } if args.is_empty() && is_builtin_type_name(name) => {
            atom_type(analyzer, name)
        }
        AstExpr::Call { name, args } if name == "capability" && args.len() == 1 => {
            let kind = symbolic_name(&args[0])?;
            lkjscript_core::CapabilityKind::parse(&kind)
                .map(Type::Capability)
                .ok_or_else(|| format!("unknown capability kind {kind}"))
        }
        AstExpr::Call { name, args } if name == "list" && args.len() == 1 => {
            Ok(Type::List(Box::new(parameter_type(analyzer, &args[0])?)))
        }
        AstExpr::Call { name, args } if name == "option" && args.len() == 1 => Ok(
            crate::types::option_type(parameter_type(analyzer, &args[0])?),
        ),
        AstExpr::Call { name, args } if name == "result" && args.len() == 2 => {
            Ok(crate::types::result_type(
                parameter_type(analyzer, &args[0])?,
                parameter_type(analyzer, &args[1])?,
            ))
        }
        AstExpr::Call { name, args } if name == "product" && args.len() == 1 => {
            let product_name = symbolic_name(&args[0])?;
            analyzer
                .product_names
                .get(&product_name)
                .copied()
                .map(Type::Product)
                .ok_or_else(|| format!("unknown product type {product_name}"))
        }
        AstExpr::Call { name, args } if is_declaration_type_name(name) => {
            let (id, parameters) = analyzer
                .enum_headers
                .get(name)
                .ok_or_else(|| format!("unknown enum type {name}"))?;
            if args.len() != parameters.len() {
                return Err(format!(
                    "enum type {name} requires {} explicit invariant arguments, got {}",
                    parameters.len(),
                    args.len()
                ));
            }
            Ok(Type::Enum {
                id: *id,
                arguments: args
                    .iter()
                    .map(|argument| parameter_type(analyzer, argument))
                    .collect::<std::result::Result<Vec<_>, _>>()?,
            })
        }
        _ => Err("invalid parameter type expression".into()),
    }
}

fn atom_type(analyzer: &Analyzer, name: &str) -> std::result::Result<Type, String> {
    if let Some((id, parameters)) = analyzer.enum_headers.get(name) {
        if parameters.is_empty() {
            return Ok(Type::Enum {
                id: *id,
                arguments: Vec::new(),
            });
        }
    }
    let (ty, end) = parse_one(&[name.to_string()], 0)?;
    if end == 1 {
        Ok(ty)
    } else {
        Err(format!("bad type {name}"))
    }
}
