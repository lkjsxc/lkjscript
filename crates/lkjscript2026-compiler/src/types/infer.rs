//! Expression type inference.

use super::ty::Type;
use crate::ast::Expr;
use lkjscript2026_core::{Error, Result};
use std::collections::HashMap;

pub fn infer_expr(env: &HashMap<String, Type>, e: &Expr) -> Result<Type> {
    match e {
        Expr::LitNil => Ok(Type::Nil),
        Expr::LitBool(_) => Ok(Type::Bool),
        Expr::LitNum(n) => Ok(if *n == (*n as i64 as f64) { Type::Int } else { Type::Float }),
        Expr::LitStr(_) => Ok(Type::Str),
        Expr::Symbol(s) => Ok(env.get(s).cloned().unwrap_or(Type::Any)),
        Expr::List(_) => Ok(Type::List),
        Expr::Call { name, args } if name == "if" => infer_if(env, args),
        Expr::Call { name, args } if name == "while" => infer_while(env, args),
        Expr::Call { name, args } if name == "do" => infer_do(env, args),
        Expr::Call { name, args } if name == "let" => infer_let(env, args),
        Expr::Call { name, args } if name == "quote" => Ok(Type::Symbol),
        Expr::Call { name, args } if name == "set" => {
            if args.len() != 2 {
                return Err(Error::msg("set needs name and value"));
            }
            let _ = infer_expr(env, &args[1])?;
            Ok(Type::Nil)
        }
        Expr::Call { name, args } if name == "bind" => {
            // bind appears only under let; type is value type
            if args.len() != 2 { return Err(Error::msg("bind needs name and value")); }
            infer_expr(env, &args[1])
        }
        Expr::Call { name, args } => infer_call(env, name, args),
    }
}

fn infer_do(env: &HashMap<String, Type>, args: &[Expr]) -> Result<Type> {
    let mut last = Type::Nil;
    for a in args { last = infer_expr(env, a)?; }
    Ok(last)
}

fn infer_while(env: &HashMap<String, Type>, args: &[Expr]) -> Result<Type> {
    if args.is_empty() { return Err(Error::msg("while needs cond")); }
    let c = infer_expr(env, &args[0])?;
    if !Type::unify_assignable(&c, &Type::Bool) {
        return Err(Error::msg("while cond must be Bool"));
    }
    for a in &args[1..] { let _ = infer_expr(env, a)?; }
    Ok(Type::Nil)
}

fn infer_let(env: &HashMap<String, Type>, args: &[Expr]) -> Result<Type> {
    if args.is_empty() { return Err(Error::msg("let needs body")); }
    let mut local = env.clone();
    let body = &args[args.len() - 1];
    for b in &args[..args.len() - 1] {
        match b {
            Expr::Call { name, args: ba } if name == "bind" && ba.len() == 2 => {
                let nm = match &ba[0] {
                    Expr::Symbol(s) => s.clone(),
                    Expr::Call { name: s, args } if args.is_empty() => s.clone(),
                    _ => return Err(Error::msg("bind name must be symbol")),
                };
                let ty = infer_expr(&local, &ba[1])?;
                local.insert(nm, ty);
            }
            _ => return Err(Error::msg("let bindings must be bind/ … /bind")),
        }
    }
    infer_expr(&local, body)
}

fn infer_if(env: &HashMap<String, Type>, args: &[Expr]) -> Result<Type> {
    if args.len() < 2 { return Err(Error::msg("if needs cond and then")); }
    let c = infer_expr(env, &args[0])?;
    if !Type::unify_assignable(&c, &Type::Bool) {
        return Err(Error::msg("if cond must be Bool"));
    }
    let t = infer_expr(env, &args[1])?;
    let e = if let Some(x) = args.get(2) { infer_expr(env, x)? } else { Type::Nil };
    if Type::unify_assignable(&t, &e) { Ok(t) }
    else if Type::unify_assignable(&e, &t) { Ok(e) }
    else { Ok(Type::Any) }
}

fn infer_call(env: &HashMap<String, Type>, name: &str, args: &[Expr]) -> Result<Type> {
    // Variadic numeric folds (matches codegen).
    if (name == "+" || name == "*") && args.len() >= 2 {
        for a in args {
            let got = infer_expr(env, a)?;
            if !Type::unify_assignable(&got, &Type::Int)
                && !Type::unify_assignable(&got, &Type::Float)
                && !Type::unify_assignable(&got, &Type::Any)
            {
                return Err(Error::msg(format!("{name}: expected number")));
            }
        }
        return Ok(Type::Int);
    }
    let fty = env.get(name).cloned().ok_or_else(|| Error::msg(format!("unbound call {name}")))?;
    match fty {
        Type::Fn { params, ret } => {
            if params.len() != args.len() {
                return Err(Error::msg(format!("{name}: expected {} args, got {}", params.len(), args.len())));
            }
            for (p, a) in params.iter().zip(args) {
                let got = infer_expr(env, a)?;
                if !Type::unify_assignable(&got, p) {
                    return Err(Error::msg(format!("{name}: arg type {got:?} not assignable to {p:?}")));
                }
            }
            Ok(*ret)
        }
        other => Err(Error::msg(format!("{name} is not a function ({other:?})"))),
    }
}
