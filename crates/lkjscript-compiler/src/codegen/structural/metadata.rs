fn structural_representation(
    chunk: &Chunk,
    ty: &SsaType,
    category: BytecodeStructuralValueCategory,
) -> Option<BytecodeStructuralRepresentationId> {
    let semantic = structural_semantic_type_from_chunk(chunk, ty)?;
    let ty = chunk
        .structural_types
        .iter()
        .find(|item| item.runtime_type.semantic_type == semantic)?;
    let type_id = ty.id;
    chunk
        .structural_representations
        .iter()
        .find(|item| item.type_id == type_id && item.category == category)
        .map(|item| item.id)
}

fn structural_field(
    program: &lkjscript_ir::Program,
    ty: &SsaType,
) -> Result<StructuralFieldMetadata> {
    let route = if let Some(item) = program.memory.type_for(ty) {
        StructuralFieldRoute::Structural(BytecodeStructuralTypeId::new(item.id.raw()))
    } else {
        field_route(ty)
    };
    let runtime_type = match route {
        StructuralFieldRoute::Resource | StructuralFieldRoute::LegacyHeap => None,
        StructuralFieldRoute::Copy
        | StructuralFieldRoute::Structural(_)
        | StructuralFieldRoute::Unique => runtime_structural_type(program, ty)?,
    };
    Ok(StructuralFieldMetadata {
        identity: field_identity(ty),
        runtime_type,
        route,
        resource: resource_kind(ty),
    })
}

fn structural_field_from_chunk(chunk: &Chunk, ty: &SsaType) -> Result<StructuralFieldMetadata> {
    let semantic = structural_semantic_type_from_chunk(chunk, ty)
        .ok_or_else(|| Error::msg("structural field semantic identity is unavailable"))?;
    let type_id = chunk
        .structural_types
        .iter()
        .find(|item| item.runtime_type.semantic_type == semantic)
        .map(|item| item.id);
    let route = type_id.map_or_else(
        || field_route(ty),
        StructuralFieldRoute::Structural,
    );
    let runtime_type = match route {
        StructuralFieldRoute::Structural(type_id) => chunk
            .structural_types
            .get(type_id.index())
            .filter(|item| item.id == type_id)
            .map(|item| item.runtime_type),
        StructuralFieldRoute::Copy | StructuralFieldRoute::Unique => {
            runtime_structural_type_without_program(ty)?
        }
        StructuralFieldRoute::Resource | StructuralFieldRoute::LegacyHeap => None,
    };
    Ok(StructuralFieldMetadata {
        identity: field_identity(ty),
        runtime_type,
        route,
        resource: resource_kind(ty),
    })
}

fn structural_semantic_type_from_chunk(
    chunk: &Chunk,
    ty: &SsaType,
) -> Option<lkjscript_core::SemanticTypeIdentity> {
    if let SsaType::Product(id) = ty {
        let product = chunk
            .products
            .iter()
            .find(|product| product.id.raw() == id.raw())?;
        return Some(lkjscript_ir::runtime_product_semantic_type(
            product.identity.bytes(),
        ));
    }
    lkjscript_ir::runtime_structural_semantic_type(None, ty).ok()
}

fn resource_kind(ty: &SsaType) -> Option<lkjscript_core::ResourceKind> {
    match ty {
        SsaType::Resource(kind) => Some(*kind),
        _ => None,
    }
}

fn field_route(ty: &SsaType) -> StructuralFieldRoute {
    match ty {
        SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64 | SsaType::Symbol => {
            StructuralFieldRoute::Copy
        }
        SsaType::Bytes | SsaType::ByteVector => StructuralFieldRoute::Unique,
        SsaType::Resource(_) => StructuralFieldRoute::Resource,
        SsaType::Str
        | SsaType::Path
        | SsaType::ByteSlice
        | SsaType::ByteSliceMut
        | SsaType::Capability(_)
        | SsaType::Product(_)
        | SsaType::Enum { .. }
        | SsaType::List(_)
        | SsaType::Function(_)
        | SsaType::TypeParameter(_)
        | SsaType::StructuralDestination(_) => StructuralFieldRoute::LegacyHeap,
    }
}

fn field_identity(ty: &SsaType) -> BytecodeLayoutId {
    let mut bytes = b"lkjscript.bytecode.structural-field\0".to_vec();
    bytes.extend_from_slice(format!("{ty:?}").as_bytes());
    BytecodeLayoutId::new(lkjscript_core::sha256(&bytes))
}

fn runtime_structural_type(
    program: &lkjscript_ir::Program,
    ty: &SsaType,
) -> Result<Option<lkjscript_core::StructuralType>> {
    lkjscript_ir::runtime_structural_type(Some(program), ty)
        .map_err(|error| Error::msg(error.to_string()))
}

fn runtime_structural_type_without_program(
    ty: &SsaType,
) -> Result<Option<lkjscript_core::StructuralType>> {
    lkjscript_ir::runtime_structural_type(None, ty)
        .map_err(|error| Error::msg(error.to_string()))
}
