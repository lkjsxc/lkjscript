use super::*;
use std::hash::{Hash, Hasher};

pub enum Type {
    /// Uninhabited, join-only control type. It is never lowered as a value.
    Never,
    Unit,
    Bool,
    I64,
    F64,
    Str,
    /// Exact immutable bytes, static or affine dynamic unique storage.
    Bytes,
    /// Exact affine owner in deterministic unique byte storage.
    ByteVector,
    /// Exact shared bounded view into a byte-vector.
    ByteSlice,
    /// Exact exclusive bounded view into a byte-vector.
    ByteSliceMut,
    Path,
    Capability(CapabilityKind),
    Symbol,
    Resource(ResourceKind),
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

impl Clone for Type {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a Type),
            Enum(EnumId, &'a str, usize),
            List,
            Function(usize),
            Forall(&'a [String]),
        }

        let mut work = vec![Work::Visit(self)];
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(ty) => match ty {
                    Type::Never => completed.push(Type::Never),
                    Type::Unit => completed.push(Type::Unit),
                    Type::Bool => completed.push(Type::Bool),
                    Type::I64 => completed.push(Type::I64),
                    Type::F64 => completed.push(Type::F64),
                    Type::Str => completed.push(Type::Str),
                    Type::Bytes => completed.push(Type::Bytes),
                    Type::ByteVector => completed.push(Type::ByteVector),
                    Type::ByteSlice => completed.push(Type::ByteSlice),
                    Type::ByteSliceMut => completed.push(Type::ByteSliceMut),
                    Type::Path => completed.push(Type::Path),
                    Type::Capability(kind) => completed.push(Type::Capability(*kind)),
                    Type::Symbol => completed.push(Type::Symbol),
                    Type::Resource(kind) => completed.push(Type::Resource(*kind)),
                    Type::Product(name) => completed.push(Type::Product(name.clone())),
                    Type::Enum {
                        id,
                        name,
                        arguments,
                    } => {
                        work.push(Work::Enum(*id, name, arguments.len()));
                        work.extend(arguments.iter().rev().map(Work::Visit));
                    }
                    Type::Param(name) => completed.push(Type::Param(name.clone())),
                    Type::List(inner) => {
                        work.push(Work::List);
                        work.push(Work::Visit(inner));
                    }
                    Type::Fn { params, ret } => {
                        work.push(Work::Function(params.len()));
                        work.push(Work::Visit(ret));
                        work.extend(params.iter().rev().map(Work::Visit));
                    }
                    Type::Forall { vars, body } => {
                        work.push(Work::Forall(vars));
                        work.push(Work::Visit(body));
                    }
                },
                Work::Enum(id, name, count) => {
                    let Some(split) = completed.len().checked_sub(count) else {
                        unreachable!("type clone enum completion order")
                    };
                    let arguments = completed.split_off(split);
                    completed.push(Type::Enum {
                        id,
                        name: name.to_owned(),
                        arguments,
                    });
                }
                Work::List => {
                    let Some(inner) = completed.pop() else {
                        unreachable!("type clone list completion order")
                    };
                    completed.push(Type::List(Box::new(inner)));
                }
                Work::Function(parameter_count) => {
                    let Some(result) = completed.pop() else {
                        unreachable!("type clone function result completion order")
                    };
                    let Some(split) = completed.len().checked_sub(parameter_count) else {
                        unreachable!("type clone function parameter completion order")
                    };
                    let params = completed.split_off(split);
                    completed.push(Type::Fn {
                        params,
                        ret: Box::new(result),
                    });
                }
                Work::Forall(vars) => {
                    let Some(body) = completed.pop() else {
                        unreachable!("type clone forall completion order")
                    };
                    completed.push(Type::Forall {
                        vars: vars.to_vec(),
                        body: Box::new(body),
                    });
                }
            }
        }
        match completed.pop() {
            Some(ty) => ty,
            None => unreachable!("type clone omitted its root"),
        }
    }
}

impl Drop for Type {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        take_children(self, &mut pending);
        while let Some(mut ty) = pending.pop() {
            take_children(&mut ty, &mut pending);
        }
    }
}

fn take_children(ty: &mut Type, pending: &mut Vec<Type>) {
    match ty {
        Type::Enum { arguments, .. } => pending.append(arguments),
        Type::List(inner) => pending.push(std::mem::replace(inner.as_mut(), Type::Unit)),
        Type::Fn { params, ret } => {
            pending.append(params);
            pending.push(std::mem::replace(ret.as_mut(), Type::Unit));
        }
        Type::Forall { body, .. } => {
            pending.push(std::mem::replace(body.as_mut(), Type::Unit));
        }
        _ => {}
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        let mut pending = vec![(self, other)];
        while let Some((left, right)) = pending.pop() {
            match (left, right) {
                (Type::Never, Type::Never)
                | (Type::Unit, Type::Unit)
                | (Type::Bool, Type::Bool)
                | (Type::I64, Type::I64)
                | (Type::F64, Type::F64)
                | (Type::Str, Type::Str)
                | (Type::Bytes, Type::Bytes)
                | (Type::ByteVector, Type::ByteVector)
                | (Type::ByteSlice, Type::ByteSlice)
                | (Type::ByteSliceMut, Type::ByteSliceMut)
                | (Type::Path, Type::Path)
                | (Type::Symbol, Type::Symbol) => {}
                (Type::Capability(left), Type::Capability(right)) if left == right => {}
                (Type::Resource(left), Type::Resource(right)) if left == right => {}
                (Type::Product(left), Type::Product(right))
                | (Type::Param(left), Type::Param(right))
                    if left == right => {}
                (
                    Type::Enum {
                        id: left_id,
                        name: left_name,
                        arguments: left_arguments,
                    },
                    Type::Enum {
                        id: right_id,
                        name: right_name,
                        arguments: right_arguments,
                    },
                ) if left_id == right_id
                    && left_name == right_name
                    && left_arguments.len() == right_arguments.len() =>
                {
                    pending.extend(left_arguments.iter().zip(right_arguments));
                }
                (Type::List(left), Type::List(right)) => pending.push((left, right)),
                (
                    Type::Fn {
                        params: left_params,
                        ret: left_ret,
                    },
                    Type::Fn {
                        params: right_params,
                        ret: right_ret,
                    },
                ) if left_params.len() == right_params.len() => {
                    pending.push((left_ret, right_ret));
                    pending.extend(left_params.iter().zip(right_params));
                }
                (
                    Type::Forall {
                        vars: left_vars,
                        body: left_body,
                    },
                    Type::Forall {
                        vars: right_vars,
                        body: right_body,
                    },
                ) if left_vars == right_vars => pending.push((left_body, right_body)),
                _ => return false,
            }
        }
        true
    }
}

impl Eq for Type {}

impl Hash for Type {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut pending = vec![self];
        while let Some(ty) = pending.pop() {
            type_tag(ty).hash(state);
            match ty {
                Type::Capability(kind) => kind.hash(state),
                Type::Resource(kind) => kind.hash(state),
                Type::Product(name) | Type::Param(name) => name.hash(state),
                Type::Enum {
                    id,
                    name,
                    arguments,
                } => {
                    id.hash(state);
                    name.hash(state);
                    arguments.len().hash(state);
                    pending.extend(arguments.iter().rev());
                }
                Type::List(inner) => pending.push(inner),
                Type::Fn { params, ret } => {
                    params.len().hash(state);
                    pending.push(ret);
                    pending.extend(params.iter().rev());
                }
                Type::Forall { vars, body } => {
                    vars.hash(state);
                    pending.push(body);
                }
                _ => {}
            }
        }
    }
}

const fn type_tag(ty: &Type) -> u8 {
    match ty {
        Type::Never => 0,
        Type::Unit => 1,
        Type::Bool => 2,
        Type::I64 => 3,
        Type::F64 => 4,
        Type::Str => 5,
        Type::Bytes => 6,
        Type::ByteVector => 7,
        Type::ByteSlice => 8,
        Type::ByteSliceMut => 9,
        Type::Path => 10,
        Type::Capability(_) => 11,
        Type::Symbol => 12,
        Type::Resource(_) => 13,
        Type::Product(_) => 14,
        Type::Enum { .. } => 15,
        Type::Param(_) => 16,
        Type::List(_) => 17,
        Type::Fn { .. } => 18,
        Type::Forall { .. } => 19,
    }
}

impl std::fmt::Debug for Type {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, formatter)
    }
}

impl Type {
    pub fn contains_never(&self) -> bool {
        let mut pending = vec![self];
        while let Some(ty) = pending.pop() {
            match ty {
                Self::Never => return true,
                Self::List(inner) => pending.push(inner),
                Self::Enum { arguments, .. } => pending.extend(arguments),
                Self::Fn { params, ret } => {
                    pending.push(ret);
                    pending.extend(params);
                }
                Self::Forall { body, .. } => pending.push(body),
                _ => {}
            }
        }
        false
    }

    pub fn join_control(left: &Type, right: &Type) -> Option<Type> {
        match (left, right) {
            (Self::Never, other) | (other, Self::Never) => Some(other.clone()),
            (left, right) if left == right => Some(left.clone()),
            _ => None,
        }
    }

    pub fn unify_assignable(got: &Type, expect: &Type) -> bool {
        let mut pending = vec![(got, expect)];
        while let Some((got, expect)) = pending.pop() {
            match (got, expect) {
                (left, right) if left == right => {}
                (Type::Param(left), Type::Param(right)) if left == right => {}
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
                ) if got_id == expected_id && got_arguments.len() == expected_arguments.len() => {
                    pending.extend(got_arguments.iter().zip(expected_arguments));
                }
                (Type::List(got), Type::List(expect)) => pending.push((got, expect)),
                (
                    Type::Fn {
                        params: got_params,
                        ret: got_ret,
                    },
                    Type::Fn {
                        params: expected_params,
                        ret: expected_ret,
                    },
                ) if got_params.len() == expected_params.len() => {
                    pending.push((got_ret, expected_ret));
                    pending.extend(got_params.iter().zip(expected_params));
                }
                _ => return false,
            }
        }
        true
    }

    pub fn subst(&self, map: &HashMap<String, Type>) -> Type {
        crate::stack::grow(|| self.subst_inner(map))
    }

    fn subst_inner(&self, map: &HashMap<String, Type>) -> Type {
        match self {
            Type::Param(parameter) => map.get(parameter).cloned().unwrap_or_else(|| self.clone()),
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
            Type::List(ty) => Type::List(Box::new(ty.subst(map))),
            Type::Fn { params, ret } => Type::Fn {
                params: params
                    .iter()
                    .map(|parameter| parameter.subst(map))
                    .collect(),
                ret: Box::new(ret.subst(map)),
            },
            Type::Forall { vars, body } => {
                let mut nested = map.clone();
                for variable in vars {
                    nested.remove(variable);
                }
                Type::Forall {
                    vars: vars.clone(),
                    body: Box::new(body.subst(&nested)),
                }
            }
            other => other.clone(),
        }
    }
}
