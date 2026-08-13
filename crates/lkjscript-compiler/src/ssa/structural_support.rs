pub(in crate::ssa) fn lower_memory_type(
    ty: &MemoryType,
    products: &HashMap<crate::hir::ProductId, ProductId>,
) -> Result<SsaType> {
    crate::stack::grow(|| lower_memory_type_inner(ty, products))
}

fn lower_memory_type_inner(
    ty: &MemoryType,
    products: &HashMap<crate::hir::ProductId, ProductId>,
) -> Result<SsaType> {
    Ok(match ty {
        MemoryType::Never => return Err(Error::msg("Never has no structural representation")),
        MemoryType::Unit => SsaType::Unit,
        MemoryType::Bool => SsaType::Bool,
        MemoryType::I64 => SsaType::I64,
        MemoryType::F64 => SsaType::F64,
        MemoryType::String => SsaType::Str,
        MemoryType::Bytes => SsaType::Bytes,
        MemoryType::Path => SsaType::Path,
        MemoryType::Capability(kind) => SsaType::Capability(*kind),
        MemoryType::ByteVector => SsaType::ByteVector,
        MemoryType::ByteSlice => SsaType::ByteSlice,
        MemoryType::ByteSliceMut => SsaType::ByteSliceMut,
        MemoryType::Symbol => SsaType::Symbol,
        MemoryType::Resource(kind) => SsaType::Resource(*kind),
        MemoryType::Product(id) => SsaType::Product(
            *products
                .get(id)
                .ok_or_else(|| Error::msg("structural type references unknown product"))?,
        ),
        MemoryType::Enum { id, arguments, .. } => SsaType::Enum {
            id: lkjscript_ir::EnumId::new(*id),
            arguments: arguments
                .iter()
                .map(|argument| lower_memory_type(argument, products))
                .collect::<Result<Vec<_>>>()?,
        },
        MemoryType::TypeParameter(name) => SsaType::TypeParameter(name.clone()),
        MemoryType::List(item) => SsaType::List(Box::new(lower_memory_type(item, products)?)),
        MemoryType::Function { parameters, result } => {
            SsaType::Function(Box::new(Signature::monomorphic(
                parameters
                    .iter()
                    .map(|parameter| lower_memory_type(parameter, products))
                    .collect::<Result<Vec<_>>>()?,
                lower_memory_type(result, products)?,
            )))
        }
        MemoryType::ForAll { variables, body } => {
            let lowered = lower_memory_type(body, products)?;
            let SsaType::Function(signature) = &lowered else {
                return Err(Error::msg("structural forall body is not callable"));
            };
            let mut signature = signature.as_ref().clone();
            signature.type_parameters = variables.clone();
            SsaType::Function(Box::new(signature))
        }
    })
}

fn layout_kind(
    product_definitions: &HashMap<hir::ProductId, &hir::ProductDefinition>,
    enum_definitions: &HashMap<[u8; 32], &hir::EnumDefinition>,
    ty: &MemoryType,
    products: &HashMap<crate::hir::ProductId, ProductId>,
) -> Result<StructuralLayoutKind> {
    match ty {
        MemoryType::String => Ok(StructuralLayoutKind::String),
        MemoryType::Path => Ok(StructuralLayoutKind::Path),
        MemoryType::Product(id) => {
            let product = product_definitions
                .get(id)
                .copied()
                .ok_or_else(|| Error::msg("structural layout lost product definition"))?;
            Ok(StructuralLayoutKind::Product {
                product: *products
                    .get(id)
                    .ok_or_else(|| Error::msg("structural layout lost ProductId"))?,
                fields: product
                    .fields
                    .iter()
                    .map(|field| lower_type(&field.ty, products))
                    .collect::<Result<Vec<_>>>()?,
            })
        }
        MemoryType::Enum { id, arguments, .. } => {
            let item = enum_definitions
                .get(id)
                .copied()
                .ok_or_else(|| Error::msg("structural layout lost enum definition"))?;
            if item.type_parameters.len() != arguments.len() {
                return Err(Error::msg("structural enum substitution arity mismatch"));
            }
            let substitutions: HashMap<_, _> = item
                .type_parameters
                .iter()
                .cloned()
                .zip(
                    arguments
                        .iter()
                        .map(|argument| lower_memory_type(argument, products))
                        .collect::<Result<Vec<_>>>()?,
                )
                .collect();
            let variants = item
                .variants
                .iter()
                .map(|variant| {
                    Ok(StructuralVariantLayout {
                        variant: lkjscript_ir::VariantId::new(variant.id.bytes()),
                        source_order: variant.source_order,
                        physical_tag: variant_physical_tag(item, variant.id)?,
                        fields: variant
                            .fields
                            .iter()
                            .map(|field| {
                                let ty = lower_type(&field.ty, products)?;
                                substitute_ssa(&ty, &substitutions)
                            })
                            .collect::<Result<Vec<_>>>()?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(StructuralLayoutKind::Enum {
                enum_id: lkjscript_ir::EnumId::new(*id),
                runtime_layout: RuntimeLayoutId::new(item.layout.identity.bytes()),
                variants,
            })
        }
        _ => Err(Error::msg(
            "non-structural type requested a structural layout",
        )),
    }
}

fn variant_physical_tag(item: &hir::EnumDefinition, id: hir::VariantId) -> Result<u64> {
    let mut ids: Vec<_> = item.variants.iter().map(|variant| variant.id).collect();
    ids.sort_by_key(|variant| variant.bytes());
    ids.iter()
        .position(|candidate| *candidate == id)
        .and_then(|index| u64::try_from(index).ok())
        .ok_or_else(|| Error::msg("structural enum physical tag is missing"))
}

fn substitute_ssa(ty: &SsaType, substitutions: &HashMap<String, SsaType>) -> Result<SsaType> {
    Ok(match ty {
        SsaType::TypeParameter(name) => substitutions
            .get(name)
            .cloned()
            .ok_or_else(|| Error::msg("structural enum field has unknown type parameter"))?,
        SsaType::Enum { id, arguments } => SsaType::Enum {
            id: *id,
            arguments: arguments
                .iter()
                .map(|argument| substitute_ssa(argument, substitutions))
                .collect::<Result<Vec<_>>>()?,
        },
        SsaType::List(item) => SsaType::List(Box::new(substitute_ssa(item, substitutions)?)),
        other => other.clone(),
    })
}
