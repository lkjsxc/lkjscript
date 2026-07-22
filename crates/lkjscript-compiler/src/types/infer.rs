//! Expression type inference (annotation-driven; instantiate forall at calls).

use super::ty::Type;
use crate::ast::Expr;
use lkjscript_core::{Error, Result};
use std::collections::HashMap;

pub fn infer_expr(env: &HashMap<String, Type>, e: &Expr) -> Result<Type> {
    match e {
        Expr::LitNil => Ok(Type::Nil),
        Expr::LitBool(_) => Ok(Type::Bool),
        Expr::LitNum {
            value: n,
            float_syntax,
        } => {
            if *float_syntax {
                Ok(Type::F64)
            } else if *n == (*n as i64 as f64) {
                Ok(Type::I64)
            } else {
                Ok(Type::F64)
            }
        }
        Expr::LitStr(_) => Ok(Type::Str),
        Expr::Symbol(s) => env
            .get(s)
            .cloned()
            .ok_or_else(|| Error::msg(format!("unbound {s}"))),
        Expr::List(_) => Err(Error::msg("raw list literal needs typed construction")),
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
            if args.len() != 2 {
                return Err(Error::msg("bind needs name and value"));
            }
            infer_expr(env, &args[1])
        }
        Expr::Call { name, args } if matches!(name.as_str(), "+" | "-" | "*" | "/" | "div")
            && args.len() >= 2 =>
        {
            infer_num_op(env, name, args)
        }
        Expr::Call { name, args }
            if matches!(
                name.as_str(),
                "=" | "!=" | "<" | "<=" | ">" | ">=" | "lt" | "le" | "gt" | "ge" | "eq" | "lte" | "gte"
            ) && args.len() == 2 =>
        {
            infer_cmp_op(env, args)
        }
        Expr::Call { name, args } => infer_call(env, name, args),
    }
}

fn infer_num_op(env: &HashMap<String, Type>, name: &str, args: &[Expr]) -> Result<Type> {
    let mut saw_f64 = false;
    let mut saw_i64 = false;
    for a in args {
        let g = infer_expr(env, a)?;
        if Type::unify_assignable(&g, &Type::F64) {
            saw_f64 = true;
        } else if Type::unify_assignable(&g, &Type::I64) {
            saw_i64 = true;
        } else {
            return Err(Error::msg(format!("{name}: expected I64 or F64, got {g:?}")));
        }
    }
    // Mixed I64/F64 promotes to F64 (explicit widths still required at bindings).
    if saw_f64 {
        Ok(Type::F64)
    } else if saw_i64 {
        Ok(Type::I64)
    } else {
        Ok(Type::I64)
    }
}

fn infer_cmp_op(env: &HashMap<String, Type>, args: &[Expr]) -> Result<Type> {
    let a = infer_expr(env, &args[0])?;
    let b = infer_expr(env, &args[1])?;
    let num = (Type::unify_assignable(&a, &Type::I64) || Type::unify_assignable(&a, &Type::F64))
        && (Type::unify_assignable(&b, &Type::I64) || Type::unify_assignable(&b, &Type::F64));
    let ok = num
        || (Type::unify_assignable(&a, &Type::Bool) && Type::unify_assignable(&b, &Type::Bool))
        || (Type::unify_assignable(&a, &Type::Str) && Type::unify_assignable(&b, &Type::Str))
        || Type::unify_assignable(&a, &b);
    if !ok {
        return Err(Error::msg(format!("compare type mismatch {a:?} vs {b:?}")));
    }
    Ok(Type::Bool)
}

fn infer_do(env: &HashMap<String, Type>, args: &[Expr]) -> Result<Type> {
    let mut last = Type::Nil;
    for a in args {
        last = infer_expr(env, a)?;
    }
    Ok(last)
}

fn infer_while(env: &HashMap<String, Type>, args: &[Expr]) -> Result<Type> {
    if args.is_empty() {
        return Err(Error::msg("while needs cond"));
    }
    let c = infer_expr(env, &args[0])?;
    if !Type::unify_assignable(&c, &Type::Bool) {
        return Err(Error::msg("while cond must be Bool"));
    }
    for a in &args[1..] {
        let _ = infer_expr(env, a)?;
    }
    Ok(Type::Nil)
}

fn infer_let(env: &HashMap<String, Type>, args: &[Expr]) -> Result<Type> {
    if args.is_empty() {
        return Err(Error::msg("let needs body"));
    }
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
    if args.len() < 2 {
        return Err(Error::msg("if needs cond and then"));
    }
    let c = infer_expr(env, &args[0])?;
    if !Type::unify_assignable(&c, &Type::Bool) {
        return Err(Error::msg("if cond must be Bool"));
    }
    let t = infer_expr(env, &args[1])?;
    let e = if let Some(x) = args.get(2) {
        infer_expr(env, x)?
    } else {
        Type::Nil
    };
    // Nil is unit for side-effect branches: join prefers the other arm.
    if matches!(t, Type::Nil) {
        Ok(e)
    } else if matches!(e, Type::Nil) {
        Ok(t)
    } else if Type::unify_assignable(&t, &e) {
        Ok(t)
    } else if Type::unify_assignable(&e, &t) {
        Ok(e)
    } else {
        Err(Error::msg(format!(
            "if branches differ: {t:?} vs {e:?}"
        )))
    }
}

fn infer_call(env: &HashMap<String, Type>, name: &str, args: &[Expr]) -> Result<Type> {
    let fty = env
        .get(name)
        .cloned()
        .ok_or_else(|| Error::msg(format!("unbound call {name}")))?;
    let fty = instantiate(name, fty, args, env)?;
    match fty {
        Type::Fn { params, ret } => {
            if params.len() != args.len() {
                return Err(Error::msg(format!(
                    "{name}: expected {} args, got {}",
                    params.len(),
                    args.len()
                )));
            }
            for (p, a) in params.iter().zip(args) {
                let got = infer_expr(env, a)?;
                if !Type::unify_assignable(&got, p) {
                    return Err(Error::msg(format!(
                        "{name}: arg type {got:?} not assignable to {p:?}"
                    )));
                }
            }
            Ok(*ret)
        }
        other => Err(Error::msg(format!("{name} is not a function ({other:?})"))),
    }
}

/// Instantiate Forall by matching argument types to parameter patterns.
fn instantiate(name: &str, fty: Type, args: &[Expr], env: &HashMap<String, Type>) -> Result<Type> {
    let Type::Forall { vars, body } = fty else {
        return Ok(fty);
    };
    let Type::Fn { params, ret } = *body else {
        return Err(Error::msg("forall body must be a function type"));
    };
    if params.len() != args.len() {
        // Still return fn type; arity checked later — but need subst empty.
        return Ok(Type::Fn { params, ret });
    }
    let mut map = HashMap::new();
    for (p, a) in params.iter().zip(args) {
        let got = infer_expr(env, a)?;
        bind_params(name, p, &got, &vars, &mut map)?;
    }
    let params = params.iter().map(|p| p.subst(&map)).collect();
    let ret = ret.subst(&map);
    Ok(Type::Fn {
        params,
        ret: Box::new(ret),
    })
}

fn bind_params(
    fname: &str,
    pattern: &Type,
    got: &Type,
    vars: &[String],
    map: &mut HashMap<String, Type>,
) -> Result<()> {
    match (pattern, got) {
        (Type::Param(p), g) if vars.iter().any(|v| v == p) => {
            if let Some(prev) = map.get(p) {
                if !Type::unify_assignable(g, prev) && !Type::unify_assignable(prev, g) {
                    return Err(Error::msg(format!(
                        "{fname}: type param {p} conflict: {prev:?} vs {g:?}"
                    )));
                }
            } else {
                map.insert(p.clone(), g.clone());
            }
            Ok(())
        }
        (Type::List(p), Type::List(g)) => bind_params(fname, p, g, vars, map),
        (Type::List(_), Type::Nil) => Ok(()), // empty list
        (Type::Option(p), Type::Option(g)) => bind_params(fname, p, g, vars, map),
        (Type::Result(a, b), Type::Result(c, d)) => {
            bind_params(fname, a, c, vars, map)?;
            bind_params(fname, b, d, vars, map)
        }
        (p, g) if Type::unify_assignable(g, p) => Ok(()),
        (p, g) => Err(Error::msg(format!(
            "{fname}: cannot instantiate {p:?} from {g:?}"
        ))),
    }
}
