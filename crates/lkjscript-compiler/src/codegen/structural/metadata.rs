fn structural_representation(
    chunk: &Chunk,
    ty: &SsaType,
    category: BytecodeStructuralValueCategory,
    storage: BytecodeStructuralStorage,
) -> Option<BytecodeStructuralRepresentationId> {
    let semantic = structural_semantic_type_from_chunk(chunk, ty)?;
    let ty = chunk
        .structural_types
        .iter()
        .find(|item| item.runtime_type.semantic_type == semantic)?;
    let type_id = ty.id;
    let mut candidates = chunk.structural_representations.iter().filter(|item| {
        item.type_id == type_id && item.category == category && item.storage == storage
    });
    let selected = candidates.next()?;
    candidates.next().is_none().then_some(selected.id)
}

pub(in crate::codegen) fn structural_owner_representation_for_value(
    function: &Function,
    chunk: &Chunk,
    value: ValueId,
) -> Option<BytecodeStructuralRepresentationId> {
    let instruction = function.blocks.iter().flat_map(|block| &block.instructions)
        .find(|instruction| instruction.id == value)?;
    match instruction.kind {
        InstructionKind::StructuralPublish { representation, .. }
        | InstructionKind::StructuralCopy { representation, .. } => {
            Some(BytecodeStructuralRepresentationId::new(representation.raw()))
        }
        InstructionKind::DestinationFinish { destination } => {
            let (representation, active_variant) =
                destination_creation(function, destination)?;
            let destination = structural_destination(chunk, representation, active_variant).ok()?;
            chunk.structural_destinations.iter()
                .find(|item| item.id == destination)
                .map(|item| item.owner_representation)
        }
        _ => None,
    }
}

fn destination_creation(
    function: &Function,
    value: ValueId,
) -> Option<(lkjscript_ir::StructuralRepresentationId, Option<lkjscript_ir::VariantId>)> {
    let instruction = function.blocks.iter().flat_map(|block| &block.instructions)
        .find(|instruction| instruction.id == value)?;
    match instruction.kind {
        InstructionKind::DestinationCreate { representation, active_variant } => {
            Some((representation, active_variant))
        }
        InstructionKind::DestinationFieldInit { destination, .. } => {
            destination_creation(function, destination)
        }
        _ => None,
    }
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
    let semantic = lkjscript_ir::runtime_structural_semantic_type(Some(program), ty)
        .map_err(|error| Error::msg(error.to_string()))?;
    Ok(StructuralFieldMetadata {
        identity: field_identity(semantic),
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
            .get_structural(type_id)
            .filter(|item| item.id == type_id)
            .map(|item| item.runtime_type),
        StructuralFieldRoute::Copy | StructuralFieldRoute::Unique => {
            runtime_structural_type_without_program(ty)?
        }
        StructuralFieldRoute::Resource | StructuralFieldRoute::LegacyHeap => None,
    };
    Ok(StructuralFieldMetadata {
        identity: field_identity(semantic),
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

fn field_identity(semantic: lkjscript_core::SemanticTypeIdentity) -> BytecodeLayoutId {
    let mut bytes =
        b"lkjscript.bytecode.structural-field\0canonical-platform-contract".to_vec();
    bytes.extend_from_slice(&32_u64.to_be_bytes());
    bytes.extend_from_slice(&semantic.get().to_be_bytes());
    BytecodeLayoutId::new(lkjscript_contracts::sha256(&bytes))
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
