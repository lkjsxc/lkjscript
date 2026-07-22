//! Walk program forms and enforce mandatory signatures.

use super::infer::infer_expr;
use super::prelude::prelude;
use super::ty::Type;
use crate::ast::Expr;
use crate::import::Program;
use lkjscript_core::{Error, Result};
use std::collections::HashMap;

pub fn typecheck_program(program: &Program) -> Result<()> {
    let mut env = prelude();
    // Pass 1: install all def signatures (forward refs).
    for file in &program.files {
        for form in &file.forms {
            if let Expr::Call { name, args } = form {
                if name == "def" {
                    install_def_sig(&mut env, args)?;
                }
            }
        }
    }
    // Pass 2: check bodies.
    for file in &program.files {
        for form in &file.forms {
            match form {
                Expr::Call { name, .. } if name == "import" => {}
                Expr::Call { name, args } if name == "def" => check_def_body(&env, args)?,
                Expr::Call { name, args } if name == "do" => {
                    for a in args {
                        let _ = infer_expr(&env, a)?;
                    }
                }
                other => return Err(Error::msg(format!("unsupported top-level: {other:?}"))),
            }
        }
    }
    Ok(())
}

fn install_def_sig(env: &mut HashMap<String, Type>, args: &[Expr]) -> Result<()> {
    match classify_def(args)? {
        DefKind::Fn { name, fn_args } => {
            let parsed = parse_fn(fn_args)?;
            if parsed.signature_params.len() != parsed.param_types.len()
                || parsed.signature_params.len() != parsed.param_names.len()
            {
                return Err(Error::msg(format!("def {name}: sig/params arity mismatch")));
            }
            let mut fty = Type::Fn {
                params: parsed.signature_params,
                ret: Box::new(parsed.signature_return),
            };
            if !parsed.forall_vars.is_empty() {
                fty = Type::Forall {
                    vars: parsed.forall_vars,
                    body: Box::new(fty),
                };
            }
            env.insert(name, fty);
            Ok(())
        }
        DefKind::Value { name, ty, .. } => {
            env.insert(name, ty);
            Ok(())
        }
    }
}

fn check_def_body(env: &HashMap<String, Type>, args: &[Expr]) -> Result<()> {
    match classify_def(args)? {
        DefKind::Fn { name, fn_args } => {
            let parsed = parse_fn(fn_args)?;
            for (signature, parameter) in parsed.signature_params.iter().zip(&parsed.param_types) {
                if !Type::unify_assignable(parameter, signature)
                    && !Type::unify_assignable(signature, parameter)
                {
                    return Err(Error::msg(format!(
                        "def {name}: param type mismatch sig vs params"
                    )));
                }
            }
            let mut local = env.clone();
            for (parameter, ty) in parsed.param_names.iter().zip(&parsed.param_types) {
                local.insert(parameter.clone(), ty.clone());
            }
            // Self ref for recursion: monomorphic version for body check.
            local.insert(
                name.clone(),
                Type::Fn {
                    params: parsed.signature_params.clone(),
                    ret: Box::new(parsed.signature_return.clone()),
                },
            );
            let got = infer_expr(&local, parsed.body)?;
            if !Type::unify_assignable(&got, &parsed.signature_return) {
                return Err(Error::msg(format!(
                    "def {name}: body type {got:?} not assignable to {:?}",
                    parsed.signature_return
                )));
            }
            Ok(())
        }
        DefKind::Value { name, ty, expr } => {
            let got = infer_expr(env, expr)?;
            if !Type::unify_assignable(&got, &ty) {
                return Err(Error::msg(format!(
                    "def {name}: value {got:?} not assignable to {ty:?}"
                )));
            }
            Ok(())
        }
    }
}

enum DefKind<'a> {
    Fn {
        name: String,
        fn_args: &'a [Expr],
    },
    Value {
        name: String,
        ty: Type,
        expr: &'a Expr,
    },
}

fn classify_def(args: &[Expr]) -> Result<DefKind<'_>> {
    let name = def_name(args)?;
    match args {
        [_, Expr::Call { name: t, args: ta }, val] if t == "type" => {
            let ty = parse_type_form(ta)?;
            Ok(DefKind::Value {
                name,
                ty,
                expr: val,
            })
        }
        [_, Expr::Call { name: f, args: fa }] if f == "fn" => Ok(DefKind::Fn { name, fn_args: fa }),
        _ => Err(Error::msg(format!(
            "def {name}: need fn/…/fn or type/…/type value"
        ))),
    }
}

fn def_name(args: &[Expr]) -> Result<String> {
    match args.first() {
        Some(Expr::Call { name: n, args: na }) if n == "name" => match na.as_slice() {
            [Expr::LitStr(s)] | [Expr::Symbol(s)] => Ok(s.clone()),
            _ => Err(Error::msg("def name must be a symbol/string")),
        },
        _ => Err(Error::msg("def expects name/ … /name first")),
    }
}

fn parse_type_form(kids: &[Expr]) -> Result<Type> {
    // type/ List I64 /type as atoms, or type/ List/ I64 /List /type
    if kids.len() == 1 {
        return param_type(&kids[0]);
    }
    let atoms = type_atoms(kids)?;
    let (t, end) = super::ty::parse_one(&atoms, 0).map_err(Error::msg)?;
    if end != atoms.len() {
        return Err(Error::msg("trailing tokens in type/"));
    }
    Ok(t)
}

struct ParsedFn<'a> {
    signature_params: Vec<Type>,
    signature_return: Type,
    param_names: Vec<String>,
    param_types: Vec<Type>,
    body: &'a Expr,
    forall_vars: Vec<String>,
}

fn parse_fn(args: &[Expr]) -> Result<ParsedFn<'_>> {
    let mut sig = None;
    let mut params = None;
    let mut body = None;
    let mut forall_vars = Vec::new();
    for a in args {
        match a {
            Expr::Call { name, args: kids } if name == "forall" => {
                for k in kids {
                    match k {
                        Expr::Symbol(s) => forall_vars.push(s.clone()),
                        Expr::Call { name, args } if args.is_empty() => {
                            forall_vars.push(name.clone())
                        }
                        _ => return Err(Error::msg("forall vars must be names")),
                    }
                }
            }
            Expr::Call { name, args: kids } if name == "sig" => sig = Some(parse_sig(kids)?),
            Expr::Call { name, args: kids } if name == "params" => {
                params = Some(parse_typed_params(kids)?)
            }
            other => {
                if body.is_some() {
                    return Err(Error::msg("fn has multiple body exprs; wrap in do/"));
                }
                body = Some(other);
            }
        }
    }
    let (sp, sr) = sig.ok_or_else(|| Error::msg("fn missing mandatory sig/ … /sig"))?;
    let (names, tys) = params.ok_or_else(|| Error::msg("fn missing params/ … /params"))?;
    let body = body.ok_or_else(|| Error::msg("fn missing body"))?;
    Ok(ParsedFn {
        signature_params: sp,
        signature_return: sr,
        param_names: names,
        param_types: tys,
        body,
        forall_vars,
    })
}

fn parse_sig(kids: &[Expr]) -> Result<(Vec<Type>, Type)> {
    let atoms = type_atoms(kids)?;
    Type::parse_atoms(&atoms).map_err(Error::msg)
}

fn type_atoms(kids: &[Expr]) -> Result<Vec<String>> {
    let mut atoms = Vec::new();
    for k in kids {
        match k {
            Expr::Symbol(s) => atoms.push(s.clone()),
            Expr::Call { name, args } if args.is_empty() => atoms.push(name.clone()),
            _ => return Err(Error::msg("type atoms must be names or ->")),
        }
    }
    Ok(atoms)
}

fn parse_typed_params(kids: &[Expr]) -> Result<(Vec<String>, Vec<Type>)> {
    if !kids.len().is_multiple_of(2) {
        return Err(Error::msg("params must be name Type pairs"));
    }
    let mut names = Vec::new();
    let mut tys = Vec::new();
    let mut i = 0;
    while i < kids.len() {
        let name = match &kids[i] {
            Expr::Symbol(s) => s.clone(),
            Expr::Call { name, args } if args.is_empty() => name.clone(),
            _ => return Err(Error::msg("param name must be a symbol")),
        };
        tys.push(param_type(&kids[i + 1])?);
        names.push(name);
        i += 2;
    }
    Ok((names, tys))
}

fn param_type(e: &Expr) -> Result<Type> {
    match e {
        Expr::Symbol(s) => atom_type(s),
        Expr::Call { name, args } if args.is_empty() => atom_type(name),
        Expr::Call { name, args } if name == "List" && args.len() == 1 => {
            Ok(Type::List(Box::new(param_type(&args[0])?)))
        }
        Expr::Call { name, args } if name == "Option" && args.len() == 1 => {
            Ok(Type::Option(Box::new(param_type(&args[0])?)))
        }
        Expr::Call { name, args } if name == "Result" && args.len() == 2 => Ok(Type::Result(
            Box::new(param_type(&args[0])?),
            Box::new(param_type(&args[1])?),
        )),
        _ => Err(Error::msg("invalid param type expr")),
    }
}

fn atom_type(s: &str) -> Result<Type> {
    let (t, end) = super::ty::parse_one(&[s.to_string()], 0).map_err(Error::msg)?;
    if end != 1 {
        return Err(Error::msg(format!("bad type {s}")));
    }
    Ok(t)
}
