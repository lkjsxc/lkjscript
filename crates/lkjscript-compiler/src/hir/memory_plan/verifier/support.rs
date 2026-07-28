use super::super::MemoryResultMode;
use super::*;

pub(super) fn callable_result(ty: &Type) -> Result<&Type> {
    match ty {
        Type::Fn { ret, .. } => Ok(ret),
        Type::Forall { body, .. } => callable_result(body),
        _ => Err(Error::msg("memory verifier expected callable HIR type")),
    }
}

pub(super) fn parameter_mode(ty: &Type, consumed: bool) -> MemoryParameterMode {
    match ty {
        Type::Owned(inner) if inner.as_ref() == &Type::Buf => MemoryParameterMode::Consume,
        Type::Ref(inner) if inner.as_ref() == &Type::Buf => MemoryParameterMode::BorrowShared,
        Type::RefMut(inner) if inner.as_ref() == &Type::Buf => MemoryParameterMode::BorrowExclusive,
        Type::Resource(_) if consumed => MemoryParameterMode::Consume,
        Type::Resource(_) => MemoryParameterMode::BorrowExclusive,
        _ => MemoryParameterMode::Copy,
    }
}

pub(super) fn result_mode(ty: &Type) -> MemoryResultMode {
    match ty {
        Type::Owned(inner) if inner.as_ref() == &Type::Buf => MemoryResultMode::Owned,
        Type::Ref(_) | Type::RefMut(_) => MemoryResultMode::Borrowed,
        Type::Resource(_) => MemoryResultMode::External,
        _ => MemoryResultMode::Trivial,
    }
}

pub(super) fn legacy_family(ty: &MemoryType) -> Option<&'static str> {
    match ty {
        MemoryType::I64 => Some("i64"),
        MemoryType::F64 => Some("f64"),
        MemoryType::String => Some("string"),
        MemoryType::Buffer | MemoryType::ByteVector => Some("buf"),
        MemoryType::Path => Some("path"),
        MemoryType::Symbol => Some("symbol"),
        MemoryType::Product(_) => Some("product"),
        MemoryType::Enum { .. } => Some("enum"),
        MemoryType::List(_) => Some("pair"),
        MemoryType::Function { .. } | MemoryType::ForAll { .. } => Some("closure"),
        _ => None,
    }
}

pub(super) fn type_matches(expected: &Type, actual: &MemoryType) -> bool {
    match (expected, actual) {
        (Type::Never, MemoryType::Never)
        | (Type::Unit, MemoryType::Unit)
        | (Type::Bool, MemoryType::Bool)
        | (Type::I64, MemoryType::I64)
        | (Type::F64, MemoryType::F64)
        | (Type::Str, MemoryType::String)
        | (Type::Buf, MemoryType::Buffer)
        | (Type::Path, MemoryType::Path)
        | (Type::Symbol, MemoryType::Symbol) => true,
        (Type::Capability(left), MemoryType::Capability(right)) => left == right,
        (Type::Owned(inner), MemoryType::ByteVector) => inner.as_ref() == &Type::Buf,
        (Type::Ref(inner), MemoryType::ByteSlice) => inner.as_ref() == &Type::Buf,
        (Type::RefMut(inner), MemoryType::ByteSliceMut) => inner.as_ref() == &Type::Buf,
        (Type::Resource(left), MemoryType::Resource(right)) => left == right,
        (Type::Product(left), MemoryType::Product(right)) => left == right,
        (
            Type::Enum {
                id,
                name,
                arguments,
            },
            MemoryType::Enum {
                id: actual_id,
                name: actual_name,
                arguments: actual_arguments,
            },
        ) => {
            id.bytes() == *actual_id
                && name == actual_name
                && type_lists_match(arguments, actual_arguments)
        }
        (Type::Param(left), MemoryType::TypeParameter(right)) => left == right,
        (Type::List(left), MemoryType::List(right)) => type_matches(left, right),
        (Type::Fn { params, ret }, MemoryType::Function { parameters, result }) => {
            type_lists_match(params, parameters) && type_matches(ret, result)
        }
        (
            Type::Forall { vars, body },
            MemoryType::ForAll {
                variables,
                body: actual_body,
            },
        ) => vars == variables && type_matches(body, actual_body),
        _ => false,
    }
}

fn type_lists_match(expected: &[Type], actual: &[MemoryType]) -> bool {
    expected.len() == actual.len()
        && expected
            .iter()
            .zip(actual)
            .all(|(left, right)| type_matches(left, right))
}
