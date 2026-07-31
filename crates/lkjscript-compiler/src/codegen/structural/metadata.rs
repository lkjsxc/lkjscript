fn structural_representation(
    chunk: &Chunk,
    ty: &SsaType,
    category: BytecodeStructuralValueCategory,
) -> Option<BytecodeStructuralRepresentationId> {
    let semantic = lkjscript_core::SemanticTypeIdentity::new(nonzero(fingerprint(
        0x8f3f_73b5_cf1c_9ade,
        ty,
    )));
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
    let semantic = lkjscript_core::SemanticTypeIdentity::new(nonzero(fingerprint(
        0x8f3f_73b5_cf1c_9ade,
        ty,
    )));
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
    runtime_structural_type_inner(Some(program), ty)
}

fn runtime_structural_type_without_program(
    ty: &SsaType,
) -> Result<Option<lkjscript_core::StructuralType>> {
    runtime_structural_type_inner(None, ty)
}

fn runtime_structural_type_inner(
    program: Option<&lkjscript_ir::Program>,
    ty: &SsaType,
) -> Result<Option<lkjscript_core::StructuralType>> {
    let kind = match ty {
        SsaType::Unit => lkjscript_core::StructuralKind::Unit,
        SsaType::Bool => lkjscript_core::StructuralKind::Bool,
        SsaType::I64 => lkjscript_core::StructuralKind::I64,
        SsaType::F64 => lkjscript_core::StructuralKind::F64,
        SsaType::Str => lkjscript_core::StructuralKind::String,
        SsaType::Path => lkjscript_core::StructuralKind::Path,
        SsaType::Bytes => lkjscript_core::StructuralKind::Bytes,
        SsaType::ByteVector => lkjscript_core::StructuralKind::ByteVector,
        SsaType::Product(_) => lkjscript_core::StructuralKind::Product,
        SsaType::Enum { .. } => lkjscript_core::StructuralKind::Enum,
        SsaType::Symbol => lkjscript_core::StructuralKind::Static,
        SsaType::ByteSlice
        | SsaType::ByteSliceMut
        | SsaType::Capability(_)
        | SsaType::Resource(_)
        | SsaType::StructuralDestination(_)
        | SsaType::List(_)
        | SsaType::Function(_)
        | SsaType::TypeParameter(_) => return Ok(None),
    };
    let semantic = fingerprint(0x8f3f_73b5_cf1c_9ade, ty);
    let layout = match ty {
        SsaType::Enum { id, .. } => {
            let program = program.ok_or_else(|| {
                Error::msg("enum field runtime identity requires installed structural metadata")
            })?;
            let definition = program
                .enums
                .iter()
                .find(|definition| definition.id == *id)
                .ok_or_else(|| Error::msg("structural enum runtime identity is missing"))?;
            fingerprint_bytes(0x9c2a_45d1_76e8_03bf, &definition.layout.identity.bytes())
        }
        SsaType::Product(id) => {
            mix(fingerprint_tag(0xe55a_7341_0a0f_b861, 12), u64::from(id.raw()))
        }
        _ => fingerprint(0x4d7c_51a9_284e_b603, ty),
    };
    Ok(Some(lkjscript_core::StructuralType::new(
        lkjscript_core::LayoutIdentity::new(nonzero(layout)),
        lkjscript_core::SemanticTypeIdentity::new(nonzero(semantic)),
        kind,
    )))
}

include!("metadata/fingerprint.rs");
