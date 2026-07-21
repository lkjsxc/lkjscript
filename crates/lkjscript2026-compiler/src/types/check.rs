//! Walk program forms and enforce mandatory signatures.

use super::infer::infer_expr;
use super::prelude::prelude;
use super::ty::Type;
use crate::ast::Expr;
use crate::import::Program;
use lkjscript2026_core::{Error, Result};
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
            let (sig_params, sig_ret, param_names, param_tys, _) = parse_fn(fn_args)?;
            if sig_params.len() != param_tys.len() || sig_params.len() != param_names.len() {
                return Err(Error::msg(format!("def {name}: sig/params arity mismatch")));
            }
            env.insert(
                name,
                Type::Fn {
                    params: sig_params,
                    ret: Box::new(sig_ret),
                },
            );
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
            let (sig_params, sig_ret, param_names, param_tys, body_expr) = parse_fn(fn_args)?;
            for (s, p) in sig_params.iter().zip(&param_tys) {
                if !Type::unify_assignable(p, s) && !Type::unify_assignable(s, p) {
                    return Err(Error::msg(format!(
                        "def {name}: param type mismatch sig vs params"
                    )));
                }
            }
            let mut local = env.clone();
            for (n, t) in param_names.iter().zip(param_tys.iter()) {
                local.insert(n.clone(), t.clone());
            }
            let got = infer_expr(&local, body_expr)?;
            if !Type::unify_assignable(&got, &sig_ret) {
                return Err(Error::msg(format!(
                    "def {name}: body type {got:?} not assignable to {sig_ret:?}"
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
            Ok(DefKind::Value { name, ty, expr: val })
        }
        [_, Expr::Call { name: f, args: fa }] if f == "fn" => Ok(DefKind::Fn {
            name,
            fn_args: fa,
        }),
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
    match kids {
        [e] => param_type(e),
        _ => Err(Error::msg("type/ expects one type")),
    }
}

fn parse_fn(args: &[Expr]) -> Result<(Vec<Type>, Type, Vec<String>, Vec<Type>, &Expr)> {
    let mut sig = None;
    let mut params = None;
    let mut body = None;
    for a in args {
        match a {
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
    Ok((sp, sr, names, tys, body))
}

fn parse_sig(kids: &[Expr]) -> Result<(Vec<Type>, Type)> {
    let mut atoms = Vec::new();
    for k in kids {
        match k {
            Expr::Symbol(s) => atoms.push(s.clone()),
            Expr::Call { name, args } if args.is_empty() => atoms.push(name.clone()),
            _ => return Err(Error::msg("sig atoms must be type names or ->")),
        }
    }
    Type::parse_atoms(&atoms).map_err(Error::msg)
}

fn parse_typed_params(kids: &[Expr]) -> Result<(Vec<String>, Vec<Type>)> {
    if kids.len() % 2 != 0 {
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
        Expr::Symbol(s) => single_type(s).ok_or_else(|| Error::msg(format!("bad type {s}"))),
        Expr::Call { name, args } if args.is_empty() => {
            single_type(name).ok_or_else(|| Error::msg(format!("bad type {name}")))
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

fn single_type(s: &str) -> Option<Type> {
    match s {
        "Nil" => Some(Type::Nil),
        "Bool" => Some(Type::Bool),
        "Int" => Some(Type::Int),
        "Float" => Some(Type::Float),
        "Str" => Some(Type::Str),
        "Buf" => Some(Type::Buf),
        "Symbol" => Some(Type::Symbol),
        "List" => Some(Type::List),
        "Handle" => Some(Type::Handle),
        "Any" => Some(Type::Any),
        _ => None,
    }
}
