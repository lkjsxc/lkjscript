use super::*;

pub(super) fn verify_dense(plan: &HirMemoryPlan) -> Result<()> {
    for (index, function) in plan.functions.iter().enumerate() {
        if function.id.raw() != index_u64(index)? || function.signature.function != function.id {
            return Err(Error::msg("HIR memory-plan functions are not dense"));
        }
    }
    for (index, entry) in plan.entries.iter().enumerate() {
        if entry.id.raw() != index_u64(index)? {
            return Err(Error::msg("HIR memory-plan entries are not dense"));
        }
    }
    for (index, item) in plan.uses.iter().enumerate() {
        if item.id.raw() != index_u64(index)? {
            return Err(Error::msg("HIR memory-plan uses are not dense"));
        }
    }
    for (index, item) in plan.constants.iter().enumerate() {
        if item.id.raw() != index_u64(index)? {
            return Err(Error::msg("HIR memory-plan constants are not dense"));
        }
    }
    for (index, item) in plan.calls.iter().enumerate() {
        if item.id.raw() != index_u64(index)? {
            return Err(Error::msg("HIR memory-plan calls are not dense"));
        }
    }
    for (index, item) in plan.obligations.iter().enumerate() {
        if item.id.raw() != index_u64(index)? {
            return Err(Error::msg("HIR memory-plan obligations are not dense"));
        }
    }
    for (index, item) in plan.type_facts.iter().enumerate() {
        if item.id.raw() != index_u64(index)? {
            return Err(Error::msg("HIR memory-plan type facts are not dense"));
        }
    }
    for (index, item) in plan.destinations.iter().enumerate() {
        if item.id.raw() != index_u64(index)? {
            return Err(Error::msg("HIR memory-plan destinations are not dense"));
        }
    }
    for (index, item) in plan.borrow_scopes.iter().enumerate() {
        if item.id.raw() != index_u64(index)? {
            return Err(Error::msg("HIR memory-plan borrow scopes are not dense"));
        }
    }
    for (index, item) in plan.drop_paths.iter().enumerate() {
        if item.id.raw() != index_u64(index)? {
            return Err(Error::msg("HIR memory-plan drop paths are not dense"));
        }
    }
    for (index, item) in plan.drop_glues.iter().enumerate() {
        if item.id.raw() != index_u64(index)? {
            return Err(Error::msg("HIR memory-plan drop glues are not dense"));
        }
    }
    Ok(())
}

pub(super) fn callable_result(ty: &Type) -> Result<&Type> {
    match ty {
        Type::Fn { ret, .. } => Ok(ret),
        Type::Forall { body, .. } => callable_result(body),
        _ => Err(Error::msg("memory verifier expected callable HIR type")),
    }
}

pub(super) fn type_matches(expected: &Type, actual: &MemoryType) -> bool {
    let mut pending = vec![(expected, actual)];
    while let Some((expected, actual)) = pending.pop() {
        match (expected, actual) {
            (Type::Never, MemoryType::Never)
            | (Type::Unit, MemoryType::Unit)
            | (Type::Bool, MemoryType::Bool)
            | (Type::I64, MemoryType::I64)
            | (Type::F64, MemoryType::F64)
            | (Type::Str, MemoryType::String)
            | (Type::Path, MemoryType::Path)
            | (Type::Symbol, MemoryType::Symbol)
            | (Type::Bytes, MemoryType::Bytes)
            | (Type::ByteVector, MemoryType::ByteVector)
            | (Type::ByteSlice, MemoryType::ByteSlice)
            | (Type::ByteSliceMut, MemoryType::ByteSliceMut) => {}
            (Type::Capability(left), MemoryType::Capability(right)) if left == right => {}
            (Type::Resource(left), MemoryType::Resource(right)) if left == right => {}
            (Type::Product(left), MemoryType::Product(right)) if left == right => {}
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
            ) if id.bytes() == *actual_id
                && name == actual_name
                && arguments.len() == actual_arguments.len() =>
            {
                pending.extend(arguments.iter().zip(actual_arguments));
            }
            (Type::Param(left), MemoryType::TypeParameter(right)) if left == right => {}
            (Type::List(left), MemoryType::List(right)) => pending.push((left, right)),
            (Type::Fn { params, ret }, MemoryType::Function { parameters, result })
                if params.len() == parameters.len() =>
            {
                pending.push((ret, result));
                pending.extend(params.iter().zip(parameters));
            }
            (
                Type::Forall { vars, body },
                MemoryType::ForAll {
                    variables,
                    body: actual_body,
                },
            ) if vars == variables => pending.push((body, actual_body)),
            _ => return false,
        }
    }
    true
}

pub(super) fn verified_memory_type(ty: &Type) -> MemoryType {
    crate::stack::grow(|| verified_memory_type_inner(ty))
}

fn verified_memory_type_inner(ty: &Type) -> MemoryType {
    match ty {
        Type::Never => MemoryType::Never,
        Type::Unit => MemoryType::Unit,
        Type::Bool => MemoryType::Bool,
        Type::I64 => MemoryType::I64,
        Type::F64 => MemoryType::F64,
        Type::Str => MemoryType::String,
        Type::Bytes => MemoryType::Bytes,
        Type::Path => MemoryType::Path,
        Type::Capability(kind) => MemoryType::Capability(*kind),
        Type::ByteVector => MemoryType::ByteVector,
        Type::ByteSlice => MemoryType::ByteSlice,
        Type::ByteSliceMut => MemoryType::ByteSliceMut,
        Type::Symbol => MemoryType::Symbol,
        Type::Resource(kind) => MemoryType::Resource(*kind),
        Type::Product(name) => MemoryType::Product(name.clone()),
        Type::Enum {
            id,
            name,
            arguments,
        } => MemoryType::Enum {
            id: id.bytes(),
            name: name.clone(),
            arguments: arguments.iter().map(verified_memory_type).collect(),
        },
        Type::Param(name) => MemoryType::TypeParameter(name.clone()),
        Type::List(inner) => MemoryType::List(Box::new(verified_memory_type(inner))),
        Type::Fn { params, ret } => MemoryType::Function {
            parameters: params.iter().map(verified_memory_type).collect(),
            result: Box::new(verified_memory_type(ret)),
        },
        Type::Forall { vars, body } => MemoryType::ForAll {
            variables: vars.clone(),
            body: Box::new(verified_memory_type(body)),
        },
    }
}
