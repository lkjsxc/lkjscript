//! Type expressions for mandatory signatures.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Nil,
    Bool,
    Int,
    Float,
    Str,
    Buf,
    Symbol,
    List,
    Handle,
    Any,
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Fn {
        params: Vec<Type>,
        ret: Box<Type>,
    },
}

impl Type {
    pub fn parse_atoms(atoms: &[String]) -> Result<(Vec<Type>, Type), String> {
        // params… -> ret
        let arrow = atoms.iter().position(|a| a == "->");
        let Some(ai) = arrow else {
            return Err("sig requires -> before return type".into());
        };
        if ai + 1 >= atoms.len() {
            return Err("sig missing return type after ->".into());
        }
        let mut params = Vec::new();
        let mut i = 0;
        while i < ai {
            let (t, next) = parse_one(atoms, i)?;
            params.push(t);
            i = next;
        }
        let (ret, end) = parse_one(atoms, ai + 1)?;
        if end != atoms.len() {
            return Err("trailing tokens after return type".into());
        }
        Ok((params, ret))
    }

    pub fn unify_assignable(got: &Type, expect: &Type) -> bool {
        if matches!(expect, Type::Any) || matches!(got, Type::Any) {
            return true;
        }
        // Empty list is Nil in this dialect.
        if matches!(got, Type::Nil) && matches!(expect, Type::List) {
            return true;
        }
        match (got, expect) {
            (a, b) if a == b => true,
            (Type::Option(g), Type::Option(e)) => Self::unify_assignable(g, e),
            (Type::Result(gok, ge), Type::Result(eok, ee)) => {
                Self::unify_assignable(gok, eok) && Self::unify_assignable(ge, ee)
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
}

fn parse_one(atoms: &[String], i: usize) -> Result<(Type, usize), String> {
    let Some(a) = atoms.get(i) else {
        return Err("expected type".into());
    };
    match a.as_str() {
        "Nil" => Ok((Type::Nil, i + 1)),
        "Bool" => Ok((Type::Bool, i + 1)),
        "Int" => Ok((Type::Int, i + 1)),
        "Float" => Ok((Type::Float, i + 1)),
        "Str" => Ok((Type::Str, i + 1)),
        "Buf" => Ok((Type::Buf, i + 1)),
        "Symbol" => Ok((Type::Symbol, i + 1)),
        "List" => Ok((Type::List, i + 1)),
        "Handle" => Ok((Type::Handle, i + 1)),
        "Any" => Ok((Type::Any, i + 1)),
        "Option" => {
            let (inner, next) = parse_one(atoms, i + 1)?;
            Ok((Type::Option(Box::new(inner)), next))
        }
        "Result" => {
            let (ok, n1) = parse_one(atoms, i + 1)?;
            let (err, n2) = parse_one(atoms, n1)?;
            Ok((Type::Result(Box::new(ok), Box::new(err)), n2))
        }
        other => Err(format!("unknown type {other}")),
    }
}
