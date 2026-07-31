fn leaf_glue(ty: &Type) -> Option<MemoryDropGlueId> {
    match ty {
        Type::ByteVector => Some(MemoryDropGlueId::new(0)),
        Type::Bytes => Some(bytes_glue()),
        Type::Resource(kind) => Some(resource_glue(*kind)),
        _ => None,
    }
}

fn type_contains_resource(ty: &Type) -> bool {
    match ty {
        Type::Resource(_) => true,
        Type::List(inner) => type_contains_resource(inner),
        Type::Enum { arguments, .. } => arguments.iter().any(type_contains_resource),
        Type::Fn { params, ret, .. } => {
            params.iter().any(type_contains_resource) || type_contains_resource(ret)
        }
        Type::Forall { body, .. } => type_contains_resource(body),
        _ => false,
    }
}

fn declaration_key(ty: &Type) -> Option<DeclarationKey> {
    match ty {
        Type::Product(name) => Some(DeclarationKey::Product(name.clone())),
        Type::Enum { id, .. } => Some(DeclarationKey::Enum(id.bytes())),
        _ => None,
    }
}

fn is_aggregate(ty: &Type) -> bool {
    matches!(ty, Type::Product(_) | Type::Enum { .. })
}
