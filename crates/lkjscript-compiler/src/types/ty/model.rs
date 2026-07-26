use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// Uninhabited, join-only control type. It is never lowered as a value.
    Never,
    Unit,
    Bool,
    I64,
    F64,
    Str,
    Buf,
    Capability(CapabilityKind),
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
    /// Nominal enum identity with invariant explicit arguments.
    Enum {
        id: EnumId,
        name: String,
        arguments: Vec<Type>,
    },
    /// Type parameter (annotation-driven polymorphism).
    Param(String),
    List(Box<Type>),
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
    pub fn contains_never(&self) -> bool {
        match self {
            Self::Never => true,
            Self::Owned(inner) | Self::Ref(inner) | Self::RefMut(inner) | Self::List(inner) => {
                inner.contains_never()
            }
            Self::Enum { arguments, .. } => arguments.iter().any(Self::contains_never),
            Self::Fn { params, ret } => {
                params.iter().any(Self::contains_never) || ret.contains_never()
            }
            Self::Forall { body, .. } => body.contains_never(),
            _ => false,
        }
    }

    pub fn join_control(left: &Type, right: &Type) -> Option<Type> {
        match (left, right) {
            (Self::Never, other) | (other, Self::Never) => Some(other.clone()),
            (left, right) if left == right => Some(left.clone()),
            _ => None,
        }
    }

    pub fn unify_assignable(got: &Type, expect: &Type) -> bool {
        match (got, expect) {
            (a, b) if a == b => true,
            (Type::Param(a), Type::Param(b)) => a == b,
            (Type::Owned(g), Type::Owned(e))
            | (Type::Ref(g), Type::Ref(e))
            | (Type::RefMut(g), Type::RefMut(e)) => Self::unify_assignable(g, e),
            (
                Type::Enum {
                    id: got_id,
                    arguments: got_arguments,
                    ..
                },
                Type::Enum {
                    id: expected_id,
                    arguments: expected_arguments,
                    ..
                },
            ) => got_id == expected_id && got_arguments == expected_arguments,
            (Type::List(g), Type::List(e)) => g == e,
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
            Type::Enum {
                id,
                name,
                arguments,
            } => Type::Enum {
                id: *id,
                name: name.clone(),
                arguments: arguments
                    .iter()
                    .map(|argument| argument.subst(map))
                    .collect(),
            },
            Type::List(t) => Type::List(Box::new(t.subst(map))),
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
