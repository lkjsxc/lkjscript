use crate::analyze::*;

pub(in crate::analyze) fn definition_name(args: &[AstExpr]) -> std::result::Result<String, String> {
    match args.first() {
        Some(AstExpr::Call { name, args }) if name == "name" => match args.as_slice() {
            [AstExpr::LitStr(name)] | [AstExpr::Symbol(name)] => Ok(name.clone()),
            [AstExpr::Call { name, args }] if args.is_empty() => Ok(name.clone()),
            _ => Err("def name must be a symbol/string".into()),
        },
        _ => Err("def expects name/…/name first".into()),
    }
}

pub(in crate::analyze) fn parse_main(
    args: &[AstExpr],
) -> std::result::Result<(Type, &AstExpr), String> {
    let [signature_form, body] = args else {
        return Err("expected exactly sig/…/sig and one body expression".into());
    };
    let AstExpr::Call {
        name,
        args: signature_args,
    } = signature_form
    else {
        return Err("expected sig/…/sig first".into());
    };
    if name != "sig" {
        return Err("expected sig/…/sig first".into());
    }
    let (params, return_type) = parse_signature(signature_args)?;
    if !params.is_empty() {
        return Err("signature must have no parameters".into());
    }
    Ok((return_type, body))
}

pub(in crate::analyze) fn parse_function(
    args: &[AstExpr],
) -> std::result::Result<ParsedFunction<'_>, String> {
    let mut index = 0;
    let mut forall_vars = Vec::new();
    if let Some(AstExpr::Call { name, args }) = args.get(index) {
        if name == "forall" {
            if args.is_empty() {
                return Err("forall must declare at least one type parameter".into());
            }
            for variable in args {
                forall_vars.push(symbolic_name(variable)?);
            }
            index += 1;
        }
    }

    let mut bounds = Vec::new();
    if let Some(AstExpr::Call { name, args }) = args.get(index) {
        if name == "bounds" {
            if args.is_empty() {
                return Err("bounds must contain at least one bound/ form".into());
            }
            for expression in args {
                let AstExpr::Call { name, args } = expression else {
                    return Err("bounds may contain only bound/ forms".into());
                };
                if name != "bound" {
                    return Err("bounds may contain only bound/ forms".into());
                }
                let [parameter, trait_name] = args.as_slice() else {
                    return Err("bound expects exactly a type parameter and trait name".into());
                };
                bounds.push(ParsedBound {
                    parameter: symbolic_name(parameter)?,
                    trait_name: symbolic_name(trait_name)?,
                });
            }
            index += 1;
        }
    }

    let signature = match args.get(index) {
        Some(AstExpr::Call { name, args }) if name == "sig" => parse_signature(args)?,
        _ => return Err("fn expects sig/ after optional forall/ and bounds/".into()),
    };
    index += 1;
    let params = match args.get(index) {
        Some(AstExpr::Call { name, args }) if name == "params" => parse_typed_params(args)?,
        _ => return Err("fn expects params/ immediately after sig/".into()),
    };
    index += 1;
    let Some(body) = args.get(index) else {
        return Err("fn missing body".into());
    };
    index += 1;
    if index != args.len() {
        return Err("fn has extra children or multiple body expressions; wrap executable expressions in do/".into());
    }
    Ok(ParsedFunction {
        signature_params: signature.0,
        signature_return: signature.1,
        param_names: params.0,
        param_types: params.1,
        body,
        forall_vars,
        bounds,
    })
}

pub(in crate::analyze) fn validate_function_header(
    name: &str,
    parsed: &ParsedFunction<'_>,
) -> std::result::Result<(), String> {
    if parsed.signature_params.len() != parsed.param_types.len()
        || parsed.signature_params.len() != parsed.param_names.len()
    {
        return Err(format!("def {name}: sig/params arity mismatch"));
    }
    let mut names = HashSet::new();
    for parameter in &parsed.param_names {
        if !names.insert(parameter) {
            return Err(format!("def {name}: duplicate parameter {parameter}"));
        }
    }
    for (signature, parameter) in parsed.signature_params.iter().zip(&parsed.param_types) {
        if signature != parameter {
            return Err(format!(
                "def {name}: parameter type mismatch between sig and params"
            ));
        }
    }

    let mut declared = HashSet::new();
    for variable in &parsed.forall_vars {
        if !declared.insert(variable.as_str()) {
            return Err(format!("def {name}: duplicate forall variable {variable}"));
        }
    }
    let mut used = HashSet::new();
    for ty in parsed
        .signature_params
        .iter()
        .chain(parsed.param_types.iter())
        .chain(std::iter::once(&parsed.signature_return))
    {
        collect_type_params(ty, &mut used);
    }
    for variable in &used {
        if !declared.contains(*variable) {
            return Err(format!(
                "def {name}: type parameter {variable} is not declared by forall"
            ));
        }
    }
    for variable in &parsed.forall_vars {
        if !used.contains(variable.as_str()) {
            return Err(format!("def {name}: unused forall variable {variable}"));
        }
    }
    Ok(())
}
