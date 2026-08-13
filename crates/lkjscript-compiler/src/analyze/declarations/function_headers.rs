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

type ParsedMain<'a> = (Vec<String>, Vec<Type>, Type, &'a AstExpr);

pub(in crate::analyze) fn parse_main<'a>(
    analyzer: &Analyzer,
    args: &'a [AstExpr],
) -> std::result::Result<ParsedMain<'a>, String> {
    let (signature_form, params_form, body) = match args {
        [signature, body] => (signature, None, body),
        [signature, params, body] => (signature, Some(params), body),
        _ => return Err("expected sig/, optional capability params/, and one body".into()),
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
    let (signature_params, return_type) = parse_signature(analyzer, signature_args)?;
    let (names, params) = match params_form {
        None if signature_params.is_empty() => (Vec::new(), Vec::new()),
        None => return Err("capability-bearing main requires params/".into()),
        Some(AstExpr::Call { name, args }) if name == "params" => {
            parse_typed_params(analyzer, args)?
        }
        Some(_) => return Err("main expects params/ immediately after sig/".into()),
    };
    if params.is_empty() && params_form.is_some() {
        return Err("pure main must omit empty params/".into());
    }
    validate_main_capabilities(&signature_params, &names, &params)?;
    Ok((names, params, return_type, body))
}

fn validate_main_capabilities(
    signature: &[Type],
    names: &[String],
    params: &[Type],
) -> std::result::Result<(), String> {
    if signature != params || names.len() != params.len() {
        return Err("main sig/params must agree exactly".into());
    }
    let mut prior = None;
    let mut seen_names = HashSet::new();
    for (name, ty) in names.iter().zip(params) {
        let Type::Capability(kind) = ty else {
            return Err("main parameters must be exact Capability values".into());
        };
        if !seen_names.insert(name) {
            return Err(format!("main has duplicate parameter {name}"));
        }
        if prior.is_some_and(|previous| previous >= *kind) {
            return Err("main capability kinds must be sorted and unique".into());
        }
        prior = Some(*kind);
    }
    Ok(())
}

pub(in crate::analyze) fn parse_function<'a>(
    analyzer: &Analyzer,
    args: &'a [AstExpr],
) -> std::result::Result<ParsedFunction<'a>, String> {
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
        Some(AstExpr::Call { name, args }) if name == "sig" => parse_signature(analyzer, args)?,
        _ => return Err("fn expects sig/ after optional forall/ and bounds/".into()),
    };
    index += 1;
    let params = match args.get(index) {
        Some(AstExpr::Call { name, args }) if name == "params" => {
            parse_typed_params(analyzer, args)?
        }
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
