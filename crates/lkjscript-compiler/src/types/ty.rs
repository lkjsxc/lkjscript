//! Type expressions: sized numerics, parametric types, no Any.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Nil,
    Bool,
    I32,
    I64,
    U32,
    U64,
    F32,
    F64,
    Str,
    Buf,
    Symbol,
    Handle,
    /// Type parameter (annotation-driven polymorphism).
    Param(String),
    List(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Fn {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    Forall {
        vars: Vec<String>,
        body: Box<Type>,
    },
}

impl Type {
    pub fn parse_atoms(atoms: &[String]) -> Result<(Vec<Type>, Type), String> {
        let arrow = atoms
            .iter()
            .position(|a| a == "->")
            .ok_or_else(|| "sig requires -> before return type".to_string())?;
        if arrow + 1 >= atoms.len() {
            return Err("sig missing return type after ->".into());
        }
        let mut params = Vec::new();
        let mut i = 0;
        while i < arrow {
            let (t, next) = parse_one(atoms, i)?;
            params.push(t);
            i = next;
        }
        let (ret, end) = parse_one(atoms, arrow + 1)?;
        if end != atoms.len() {
            return Err("trailing tokens after return type".into());
        }
        Ok((params, ret))
    }

    pub fn unify_assignable(got: &Type, expect: &Type) -> bool {
        // Empty list.
        if matches!(got, Type::Nil) {
            if let Type::List(_) = expect {
                return true;
            }
            // Uninitialized buffer/handle slots may be nil.
            if matches!(expect, Type::Buf | Type::Handle) {
                return true;
            }
        }
        match (got, expect) {
            (a, b) if a == b => true,
            (Type::Param(a), Type::Param(b)) => a == b,
            (Type::List(g), Type::List(e)) => Self::unify_assignable(g, e),
            (Type::Option(g), Type::Option(e)) => Self::unify_assignable(g, e),
            (Type::Result(a, b), Type::Result(c, d)) => {
                Self::unify_assignable(a, c) && Self::unify_assignable(b, d)
            }
            (
                Type::Fn {
                    params: gp,
                    ret: gr,
                },
                Type::Fn {
                    params: ep,
                    ret: er,
                },
            ) if gp.len() == ep.len() => {
                gp.iter()
                    .zip(ep)
                    .all(|(g, e)| Self::unify_assignable(g, e))
                    && Self::unify_assignable(gr, er)
            }
            _ => false,
        }
    }

    pub fn subst(&self, map: &HashMap<String, Type>) -> Type {
        match self {
            Type::Param(p) => map.get(p).cloned().unwrap_or_else(|| self.clone()),
            Type::List(t) => Type::List(Box::new(t.subst(map))),
            Type::Option(t) => Type::Option(Box::new(t.subst(map))),
            Type::Result(a, b) => Type::Result(Box::new(a.subst(map)), Box::new(b.subst(map))),
            Type::Fn { params, ret } => Type::Fn {
                params: params.iter().map(|p| p.subst(map)).collect(),
                ret: Box::new(ret.subst(map)),
            },
            Type::Forall { vars, body } => {
                let mut m = map.clone();
                for v in vars {
                    m.remove(v);
                }
                Type::Forall {
                    vars: vars.clone(),
                    body: Box::new(body.subst(&m)),
                }
            }
            other => other.clone(),
        }
    }
}

pub fn parse_one(atoms: &[String], i: usize) -> Result<(Type, usize), String> {
    let Some(a) = atoms.get(i) else {
        return Err("expected type".into());
    };
    match a.as_str() {
        "Any" => Err("Any is not a permitted type".into()),
        "Nil" => Ok((Type::Nil, i + 1)),
        "Bool" => Ok((Type::Bool, i + 1)),
        "I32" | "i32" => Ok((Type::I32, i + 1)),
        "I64" | "i64" | "Int" => Ok((Type::I64, i + 1)), // Int alias → I64
        "U32" | "u32" => Ok((Type::U32, i + 1)),
        "U64" | "u64" => Ok((Type::U64, i + 1)),
        "F32" | "f32" => Ok((Type::F32, i + 1)),
        "F64" | "f64" | "Float" => Ok((Type::F64, i + 1)),
        "Str" => Ok((Type::Str, i + 1)),
        "Buf" => Ok((Type::Buf, i + 1)),
        "Symbol" => Ok((Type::Symbol, i + 1)),
        "Handle" => Ok((Type::Handle, i + 1)),
        "List" => {
            let (inner, next) = parse_one(atoms, i + 1)?;
            Ok((Type::List(Box::new(inner)), next))
        }
        "Option" => {
            let (inner, next) = parse_one(atoms, i + 1)?;
            Ok((Type::Option(Box::new(inner)), next))
        }
        "Result" => {
            let (ok, n1) = parse_one(atoms, i + 1)?;
            let (err, n2) = parse_one(atoms, n1)?;
            Ok((Type::Result(Box::new(ok), Box::new(err)), n2))
        }
        // Type parameter: single uppercase letter or T, U, E, …
        other if is_type_param_name(other) => Ok((Type::Param(other.to_string()), i + 1)),
        other => Err(format!("unknown type {other}")),
    }
}

fn is_type_param_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && s.chars().all(|c| c.is_ascii_alphanumeric())
        && !matches!(
            s,
            "Nil" | "Bool"
                | "I32"
                | "I64"
                | "U32"
                | "U64"
                | "F32"
                | "F64"
                | "Str"
                | "Buf"
                | "Symbol"
                | "Handle"
                | "List"
                | "Option"
                | "Result"
                | "Int"
                | "Float"
                | "Any"
        )
}
