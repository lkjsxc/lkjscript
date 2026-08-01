pub(in crate::codegen) fn install_structural_metadata(
    chunk: &mut Chunk,
    program: &lkjscript_ir::Program,
) -> Result<()> {
    let install_structural_routes = requirements::structural_routes(program);
    let install_executable_witnesses =
        requirements::executable_witnesses(program, install_structural_routes);
    if install_executable_witnesses {
        install_memory_witnesses(chunk, program, install_structural_routes)?;
    }
    if !install_structural_routes {
        if !program.region_products.is_empty() {
            chunk.memory_plan = Some(BytecodeMemoryPlanId::new(program.memory.plan.bytes()));
        }
        return Ok(());
    }
    if program.memory.types.is_empty() {
        if !program.memory.layouts.is_empty() || !program.memory.representations.is_empty() {
            return Err(Error::msg("SSA structural metadata tables are inconsistent"));
        }
        return Ok(());
    }
    chunk.memory_plan = Some(BytecodeMemoryPlanId::new(program.memory.plan.bytes()));
    for item in &program.memory.types {
        if item.id.index() != Some(chunk.structural_types.len()) {
            return Err(Error::msg(
                "SSA structural TypeIds are not dense during bytecode lowering",
            ));
        }
        let kind = match &item.ty {
            SsaType::Str => BytecodeStructuralTypeKind::String,
            SsaType::Path => BytecodeStructuralTypeKind::Path,
            SsaType::Product(product) => {
                BytecodeStructuralTypeKind::Product(BytecodeProductId::new(product.raw()))
            }
            SsaType::Enum { id, .. } => {
                BytecodeStructuralTypeKind::Enum(lkjscript_core::EnumId::new(id.bytes()))
            }
            _ => {
                return Err(Error::msg(
                    "SSA structural type has no bytecode type identity",
                ))
            }
        };
        chunk.structural_types.push(BytecodeStructuralTypeMetadata {
            id: BytecodeStructuralTypeId::new(item.id.raw()),
            witness: BytecodeMemoryWitnessId::new(item.witness.bytes()),
            identity: field_identity(&item.ty),
            runtime_type: runtime_structural_type(program, &item.ty)?.ok_or_else(|| {
                Error::msg("SSA structural type has no exact value-runtime identity")
            })?,
            kind,
            layout: BytecodeStructuralLayoutId::new(item.layout.raw()),
            mode: match item.mode {
                lkjscript_ir::StructuralTypeMode::Copy => BytecodeStructuralTypeMode::Copy,
                lkjscript_ir::StructuralTypeMode::Immutable => {
                    BytecodeStructuralTypeMode::Immutable
                }
                lkjscript_ir::StructuralTypeMode::Affine => BytecodeStructuralTypeMode::Affine,
            },
        });
    }
    for layout in &program.memory.layouts {
        if layout.id.index() != Some(chunk.structural_layouts.len()) {
            return Err(Error::msg(
                "SSA structural LayoutIds are not dense during bytecode lowering",
            ));
        }
        let kind = match &layout.kind {
            lkjscript_ir::StructuralLayoutKind::String => BytecodeStructuralLayoutKind::String,
            lkjscript_ir::StructuralLayoutKind::Path => BytecodeStructuralLayoutKind::Path,
            lkjscript_ir::StructuralLayoutKind::Product { product, fields } => {
                BytecodeStructuralLayoutKind::Product {
                    product: BytecodeProductId::new(product.raw()),
                    fields: fields
                        .iter()
                        .map(|field| structural_field(program, field))
                        .collect::<Result<Vec<_>>>()?,
                }
            }
            lkjscript_ir::StructuralLayoutKind::Enum {
                enum_id,
                runtime_layout,
                variants,
            } => BytecodeStructuralLayoutKind::Enum {
                enum_id: lkjscript_core::EnumId::new(enum_id.bytes()),
                runtime_layout: BytecodeLayoutId::new(runtime_layout.bytes()),
                variants: variants
                    .iter()
                    .map(|variant| {
                        Ok(BytecodeStructuralVariantLayout {
                            variant: BytecodeVariantId::new(variant.variant.bytes()),
                            physical_tag: variant.physical_tag,
                            fields: variant
                                .fields
                                .iter()
                                .map(|field| structural_field(program, field))
                                .collect::<Result<Vec<_>>>()?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
        };
        chunk
            .structural_layouts
            .push(BytecodeStructuralLayoutMetadata {
                id: BytecodeStructuralLayoutId::new(layout.id.raw()),
                identity: BytecodeLayoutId::new(layout.identity.bytes()),
                kind,
            });
    }
    for representation in &program.memory.representations {
        if representation.id.index() != Some(chunk.structural_representations.len()) {
            return Err(Error::msg(
                "SSA structural RepresentationIds are not dense during bytecode lowering",
            ));
        }
        chunk
            .structural_representations
            .push(BytecodeStructuralRepresentationMetadata {
                id: BytecodeStructuralRepresentationId::new(representation.id.raw()),
                type_id: BytecodeStructuralTypeId::new(representation.type_id.raw()),
                layout: BytecodeStructuralLayoutId::new(representation.layout.raw()),
                category: match representation.category {
                    StructuralValueCategory::Owner => BytecodeStructuralValueCategory::Owner,
                    StructuralValueCategory::View => BytecodeStructuralValueCategory::View,
                    StructuralValueCategory::Destination => {
                        BytecodeStructuralValueCategory::Destination
                    }
                },
                storage: match representation.storage {
                    lkjscript_ir::StructuralStorage::Static => BytecodeStructuralStorage::Static,
                    lkjscript_ir::StructuralStorage::Unique => BytecodeStructuralStorage::Unique,
                    lkjscript_ir::StructuralStorage::Stack => BytecodeStructuralStorage::Stack,
                    lkjscript_ir::StructuralStorage::CallerDestination => {
                        BytecodeStructuralStorage::CallerDestination
                    }
                },
            });
    }
    let destination_representations: Vec<_> = chunk
        .structural_representations
        .iter()
        .filter(|item| item.category == BytecodeStructuralValueCategory::Destination)
        .copied()
        .collect();
    for representation in destination_representations {
        let owner_representation = chunk
            .structural_representations
            .iter()
            .find(|item| {
                item.type_id == representation.type_id
                    && item.category == BytecodeStructuralValueCategory::Owner
            })
            .map(|item| item.id)
            .ok_or_else(|| Error::msg("structural destination has no owner representation"))?;
        let layout = chunk
            .structural_layouts
            .get(representation.layout.index())
            .ok_or_else(|| Error::msg("structural destination layout is missing"))?;
        let candidates: Vec<(Option<BytecodeVariantId>, Vec<StructuralFieldMetadata>)> =
            match &layout.kind {
                BytecodeStructuralLayoutKind::String | BytecodeStructuralLayoutKind::Path => {
                    vec![(None, Vec::new())]
                }
                BytecodeStructuralLayoutKind::Product { fields, .. } => {
                    vec![(None, fields.clone())]
                }
                BytecodeStructuralLayoutKind::Enum { variants, .. } => variants
                    .iter()
                    .map(|variant| (Some(variant.variant), variant.fields.clone()))
                    .collect(),
            };
        for (active_variant, fields) in candidates {
            let raw = u16::try_from(chunk.structural_destinations.len())
                .map_err(|_| Error::msg("bytecode structural destinations exceed u16"))?;
            chunk
                .structural_destinations
                .push(StructuralDestinationMetadata {
                    id: StructuralDestinationId::new(raw),
                    representation: representation.id,
                    owner_representation,
                    active_variant,
                    fields,
                });
        }
    }
    Ok(())
}
