use crate::source::{Expr, SourceDiagnostic, SourceFile, SourceResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublicMemoryWitnessRequirement {
    pub(crate) export: String,
    pub(crate) parameter: String,
    pub(crate) operations: Vec<String>,
}

pub(crate) fn public_memory_witness_requirements(
    file: &SourceFile,
) -> SourceResult<Vec<PublicMemoryWitnessRequirement>> {
    let exports = super::interface(file)?.exports;
    let mut output = Vec::new();
    for form in &file.forms {
        let Expr::Call { name, args } = form else {
            continue;
        };
        if name != "def" {
            continue;
        }
        let Some(export) = named(args) else {
            continue;
        };
        if !exports.contains(export) {
            continue;
        }
        let Some(Expr::Call { args: function, .. }) =
            args.iter().find(|item| call_name(item) == Some("fn"))
        else {
            continue;
        };
        let Some(Expr::Call {
            args: variables, ..
        }) = function
            .iter()
            .find(|item| call_name(item) == Some("forall"))
        else {
            continue;
        };
        let variables = variables
            .iter()
            .map(|item| match item {
                Expr::Symbol(name) => Ok(name.as_str()),
                _ => Err(error(
                    file,
                    "public generic has malformed forall parameters",
                )),
            })
            .collect::<SourceResult<Vec<_>>>()?;
        let signature = function
            .iter()
            .find(|item| call_name(item) == Some("sig"))
            .ok_or_else(|| error(file, "public generic has no signature"))?;
        let Expr::Call {
            args: signature, ..
        } = signature
        else {
            unreachable!("call name requires call expression")
        };
        let inputs = call_args(signature, "inputs")
            .ok_or_else(|| error(file, "public generic has malformed inputs"))?;
        let result = call_args(signature, "output")
            .and_then(|items| matches!(items, [_]).then_some(&items[0]))
            .ok_or_else(|| error(file, "public generic has malformed output"))?;
        for parameter in variables {
            let naked = inputs.iter().any(|ty| symbol(ty) == Some(parameter))
                || symbol(result) == Some(parameter);
            let nested = inputs
                .iter()
                .chain(std::iter::once(result))
                .any(|ty| symbol(ty) != Some(parameter) && contains_symbol(ty, parameter));
            if nested {
                return Err(error(
                    file,
                    "public generic has unsupported nested memory-witness use",
                ));
            }
            if naked {
                output.push(PublicMemoryWitnessRequirement {
                    export: export.to_owned(),
                    parameter: parameter.to_owned(),
                    operations: vec!["transport".into()],
                });
            }
        }
    }
    output.sort_by(|left, right| {
        (&left.export, &left.parameter).cmp(&(&right.export, &right.parameter))
    });
    Ok(output)
}

fn named(args: &[Expr]) -> Option<&str> {
    let values = call_args(args, "name")?;
    match values {
        [Expr::LitStr(name)] => Some(name),
        _ => None,
    }
}

fn call_args<'a>(args: &'a [Expr], name: &str) -> Option<&'a [Expr]> {
    args.iter().find_map(|item| match item {
        Expr::Call { name: item, args } if item == name => Some(args.as_slice()),
        _ => None,
    })
}

fn call_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Call { name, .. } => Some(name),
        _ => None,
    }
}

fn symbol(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Symbol(name) => Some(name),
        _ => None,
    }
}

fn contains_symbol(expr: &Expr, expected: &str) -> bool {
    match expr {
        Expr::Symbol(name) => name == expected,
        Expr::Call { args, .. } | Expr::List(args) => {
            args.iter().any(|item| contains_symbol(item, expected))
        }
        _ => false,
    }
}

fn error(file: &SourceFile, message: &str) -> SourceDiagnostic {
    SourceDiagnostic::generic(file.origin.clone(), message)
}
