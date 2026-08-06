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
        let runtime_type = runtime_structural_type(program, &item.ty)?
            .ok_or_else(|| Error::msg("SSA structural type has no exact value-runtime identity"))?;
        chunk.structural_types.push(BytecodeStructuralTypeMetadata {
            id: BytecodeStructuralTypeId::new(item.id.raw()),
            witness: BytecodeMemoryWitnessId::new(item.witness.bytes()),
            identity: field_identity(runtime_type.semantic_type),
            runtime_type,
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
                            source_order: variant.source_order,
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
                witness: BytecodeMemoryWitnessId::new(representation.witness.bytes()),
                witness_group: lkjscript_core::MemoryWitnessGroupId::new(
                    representation.witness_group.bytes(),
                ),
                witness_member: representation.witness_member,
                layout: BytecodeStructuralLayoutId::new(representation.layout.raw()),
                category: match representation.category {
                    StructuralValueCategory::Owner => BytecodeStructuralValueCategory::Owner,
                    StructuralValueCategory::View => BytecodeStructuralValueCategory::View,
                    StructuralValueCategory::Destination => {
                        BytecodeStructuralValueCategory::Destination
                    }
                },
                storage: match representation.storage {
                    lkjscript_ir::StructuralStorage::Inline => BytecodeStructuralStorage::Inline,
                    lkjscript_ir::StructuralStorage::Static => BytecodeStructuralStorage::Static,
                    lkjscript_ir::StructuralStorage::Stack => BytecodeStructuralStorage::Stack,
                    lkjscript_ir::StructuralStorage::CallerDestination => BytecodeStructuralStorage::CallerDestination,
                    lkjscript_ir::StructuralStorage::UniqueStructural => BytecodeStructuralStorage::UniqueStructural,
                    lkjscript_ir::StructuralStorage::OrdinaryRegion => BytecodeStructuralStorage::OrdinaryRegion,
                    lkjscript_ir::StructuralStorage::SealedRegion => BytecodeStructuralStorage::SealedRegion,
                    lkjscript_ir::StructuralStorage::BorrowedView => BytecodeStructuralStorage::BorrowedView,
                    lkjscript_ir::StructuralStorage::ExternalResource => BytecodeStructuralStorage::ExternalResource,
                },
                route: representation.route,
            });
    }
    install_structural_destinations(chunk)
}
