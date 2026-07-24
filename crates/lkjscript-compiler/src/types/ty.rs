//! Type expressions: exact I64/F64 numerics, parametric types, no Any.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Unit,
    Bool,
    I64,
    F64,
    Str,
    Buf,
    /// Initial ownership slice: only `Owned Buf` is well formed.
    Owned(Box<Type>),
    /// Initial ownership slice: only `Ref Buf` is well formed.
    Ref(Box<Type>),
    /// Initial ownership slice: only `RefMut Buf` is well formed.
    RefMut(Box<Type>),
    Symbol,
    Handle,
    /// Globally unique nominal product declaration name.
    Product(String),
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
        match (got, expect) {
            (a, b) if a == b => true,
            (Type::Param(a), Type::Param(b)) => a == b,
            (Type::Owned(g), Type::Owned(e))
            | (Type::Ref(g), Type::Ref(e))
            | (Type::RefMut(g), Type::RefMut(e)) => Self::unify_assignable(g, e),
            (Type::List(g), Type::List(e)) => g == e,
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
                gp.iter().zip(ep).all(|(g, e)| Self::unify_assignable(g, e))
                    && Self::unify_assignable(gr, er)
            }
            _ => false,
        }
    }

    pub fn subst(&self, map: &HashMap<String, Type>) -> Type {
        match self {
            Type::Param(p) => map.get(p).cloned().unwrap_or_else(|| self.clone()),
            Type::Owned(t) => Type::Owned(Box::new(t.subst(map))),
            Type::Ref(t) => Type::Ref(Box::new(t.subst(map))),
            Type::RefMut(t) => Type::RefMut(Box::new(t.subst(map))),
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
        "Unit" => Ok((Type::Unit, i + 1)),
        "Nil" => Err("Nil was removed; use Unit, Option T, or List T".into()),
        "Bool" => Ok((Type::Bool, i + 1)),
        "I64" => Ok((Type::I64, i + 1)),
        "F64" => Ok((Type::F64, i + 1)),
        "I32" | "U32" | "U64" | "F32" | "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "Int"
        | "Float" => Err(format!(
            "unsupported numeric type {a}; use canonical I64 or F64"
        )),
        "Str" => Ok((Type::Str, i + 1)),
        "Buf" => Ok((Type::Buf, i + 1)),
        "Owned" | "Ref" | "RefMut" => {
            let (inner, next) = parse_one(atoms, i + 1)?;
            if inner != Type::Buf {
                return Err(format!(
                    "{a} accepts only exact Buf in the initial ownership slice"
                ));
            }
            let ty = match a.as_str() {
                "Owned" => Type::Owned(Box::new(inner)),
                "Ref" => Type::Ref(Box::new(inner)),
                "RefMut" => Type::RefMut(Box::new(inner)),
                _ => return Err("invalid ownership type".into()),
            };
            Ok((ty, next))
        }
        "Symbol" => Ok((Type::Symbol, i + 1)),
        "Handle" => Ok((Type::Handle, i + 1)),
        "Product" => {
            let Some(name) = atoms.get(i + 1) else {
                return Err("Product requires a declared product name".into());
            };
            if !is_product_type_name(name) {
                return Err(format!("invalid product type name {name}"));
            }
            Ok((Type::Product(name.clone()), i + 2))
        }
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
        other if is_numeric_width_name(other) => Err(format!(
            "unsupported numeric type {other}; use canonical I64 or F64"
        )),
        // Type parameter: single uppercase letter or T, U, E, …
        other if is_type_param_name(other) => Ok((Type::Param(other.to_string()), i + 1)),
        other => Err(format!("unknown type {other}")),
    }
}

fn is_numeric_width_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b'I' | b'U' | b'F' | b'i' | b'u' | b'f'))
        && bytes.clone().next().is_some()
        && bytes.all(|byte| byte.is_ascii_digit())
}

fn is_type_param_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && s.chars().all(|c| c.is_ascii_alphanumeric())
        && !matches!(
            s,
            "Unit"
                | "Bool"
                | "I64"
                | "F64"
                | "Str"
                | "Buf"
                | "Owned"
                | "Ref"
                | "RefMut"
                | "Symbol"
                | "Handle"
                | "List"
                | "Option"
                | "Result"
                | "Product"
                | "Int"
                | "Float"
                | "Any"
        )
}

fn is_product_type_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

#[cfg(test)]
mod tests {
    use super::{parse_one, Type};

    #[test]
    fn product_types_are_explicit_and_nominal_by_name() {
        let atoms = vec!["List".into(), "Product".into(), "Point".into()];
        assert_eq!(
            parse_one(&atoms, 0),
            Ok((Type::List(Box::new(Type::Product("Point".into()))), 3))
        );
        assert!(parse_one(&["Product".into()], 0).is_err());
        assert!(parse_one(&["Product".into(), "lowercase".into()], 0).is_err());
    }

    #[test]
    fn only_canonical_numeric_type_names_are_accepted() {
        for (name, expected) in [("I64", Type::I64), ("F64", Type::F64)] {
            assert_eq!(parse_one(&[name.into()], 0).ok(), Some((expected, 1)));
        }
        for name in [
            "I32", "U32", "U64", "F32", "I128", "U8", "F16", "i32", "i64", "u32", "u64", "f32",
            "f64", "i128", "Int", "Float",
        ] {
            assert!(parse_one(&[name.into()], 0).is_err(), "accepted {name}");
        }
    }
}
