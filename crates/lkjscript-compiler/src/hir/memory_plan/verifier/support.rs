use super::*;

pub(super) fn verify_dense(plan: &HirMemoryPlan) -> Result<()> {
    for (index, function) in plan.functions.iter().enumerate() {
        if function.id.raw() != index_u32(index)? || function.signature.function != function.id {
            return Err(Error::msg("HIR memory-plan functions are not dense"));
        }
    }
    for (index, entry) in plan.entries.iter().enumerate() {
        if entry.id.raw() != index_u32(index)? {
            return Err(Error::msg("HIR memory-plan entries are not dense"));
        }
    }
    for (index, item) in plan.uses.iter().enumerate() {
        if item.id.raw() != index_u32(index)? {
            return Err(Error::msg("HIR memory-plan uses are not dense"));
        }
    }
    for (index, item) in plan.constants.iter().enumerate() {
        if item.id.raw() != index_u32(index)? {
            return Err(Error::msg("HIR memory-plan constants are not dense"));
        }
    }
    for (index, item) in plan.calls.iter().enumerate() {
        if item.id.raw() != index_u32(index)? {
            return Err(Error::msg("HIR memory-plan calls are not dense"));
        }
    }
    for (index, item) in plan.obligations.iter().enumerate() {
        if item.id.raw() != index_u32(index)? {
            return Err(Error::msg("HIR memory-plan obligations are not dense"));
        }
    }
    for (index, item) in plan.type_facts.iter().enumerate() {
        if item.id.raw() != index_u32(index)? {
            return Err(Error::msg("HIR memory-plan type facts are not dense"));
        }
    }
    for (index, item) in plan.destinations.iter().enumerate() {
        if item.id.raw() != index_u32(index)? {
            return Err(Error::msg("HIR memory-plan destinations are not dense"));
        }
    }
    for (index, item) in plan.borrow_scopes.iter().enumerate() {
        if item.id.raw() != index_u32(index)? {
            return Err(Error::msg("HIR memory-plan borrow scopes are not dense"));
        }
    }
    for (index, item) in plan.drop_paths.iter().enumerate() {
        if item.id.raw() != index_u32(index)? {
            return Err(Error::msg("HIR memory-plan drop paths are not dense"));
        }
    }
    for (index, item) in plan.drop_glues.iter().enumerate() {
        if item.id.raw() != index_u32(index)? {
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
    match (expected, actual) {
        (Type::Never, MemoryType::Never)
        | (Type::Unit, MemoryType::Unit)
        | (Type::Bool, MemoryType::Bool)
        | (Type::I64, MemoryType::I64)
        | (Type::F64, MemoryType::F64)
        | (Type::Str, MemoryType::String)
        | (Type::Path, MemoryType::Path)
        | (Type::Symbol, MemoryType::Symbol) => true,
        (Type::Capability(left), MemoryType::Capability(right)) => left == right,
        (Type::Bytes, MemoryType::Bytes)
        | (Type::ByteVector, MemoryType::ByteVector)
        | (Type::ByteSlice, MemoryType::ByteSlice)
        | (Type::ByteSliceMut, MemoryType::ByteSliceMut) => true,
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

pub(super) fn verified_memory_type(ty: &Type) -> MemoryType {
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
