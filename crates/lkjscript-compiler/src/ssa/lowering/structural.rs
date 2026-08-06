use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn publish_structural_source(
        &mut self,
        ty: SsaType,
        source: ValueId,
        expression_origin: hir::SourceId,
    ) -> Result<ValueId> {
        let Some(representation) = self
            .selected_structural_representation(&ty, StructuralValueCategory::Owner)
            .filter(|_| self.structural.is_owned(&ty))
        else {
            return Ok(source);
        };
        self.append(
            ty,
            InstructionKind::StructuralPublish {
                representation,
                value: source,
            },
            EffectSet::ALLOCATES,
            expression_origin,
        )
    }

    pub(in crate::ssa) fn construct_structural_aggregate(
        &mut self,
        ty: SsaType,
        active_variant: Option<lkjscript_ir::VariantId>,
        fields: Vec<ValueId>,
        expression_origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let Some(type_fact) = self.structural.type_for(&ty) else {
            return Ok(None);
        };
        let representation = self
            .selected_structural_representation(&ty, StructuralValueCategory::Destination)
            .ok_or_else(|| {
                Error::msg("selected structural aggregate has no exact destination representation")
            })?;
        let destination_ty = SsaType::StructuralDestination(type_fact.id);
        let mut destination = self.append(
            destination_ty.clone(),
            InstructionKind::DestinationCreate {
                representation,
                active_variant,
            },
            EffectSet::ALLOCATES,
            expression_origin,
        )?;
        for (index, value) in fields.into_iter().enumerate() {
            let field = u64::try_from(index)
                .map_err(|_| Error::msg("structural destination field exceeds u64"))?;
            destination = self.append(
                destination_ty.clone(),
                InstructionKind::DestinationFieldInit {
                    destination,
                    field,
                    value,
                },
                EffectSet::WRITES_MEMORY.union(EffectSet::ALLOCATES),
                expression_origin,
            )?;
        }
        self.append(
            ty,
            InstructionKind::DestinationFinish { destination },
            EffectSet::ALLOCATES,
            expression_origin,
        )
        .map(Some)
    }

    pub(in crate::ssa) fn structural_representation(
        &self,
        ty: &SsaType,
        category: StructuralValueCategory,
    ) -> Result<StructuralRepresentationId> {
        self.selected_structural_representation(ty, category)
            .ok_or_else(|| {
                Error::msg(format!(
                    "selected structural type {ty:?} has no exact {category:?} representation"
                ))
            })
    }

    fn selected_structural_representation(
        &self,
        ty: &SsaType,
        category: StructuralValueCategory,
    ) -> Option<StructuralRepresentationId> {
        if let Some(placement) = self.current_placement {
            if let Some(id) = self.structural.representation_by_route(
                ty,
                placement.route,
                category,
                placement.storage,
            ) {
                return Some(id);
            }
        }
        let type_fact = self.structural.type_for(ty)?;
        let (storage, route_tag, value_category) = match category {
            StructuralValueCategory::Owner | StructuralValueCategory::Destination => (
                StructuralStorage::UniqueStructural,
                2,
                crate::memory_plan::MemoryValueCategory::Owner,
            ),
            StructuralValueCategory::View => (
                StructuralStorage::BorrowedView,
                0,
                crate::memory_plan::MemoryValueCategory::View,
            ),
        };
        let route =
            canonical_structural_route(type_fact.witness, value_category, storage, route_tag);
        self.structural
            .representation_by_route(ty, route, category, storage)
    }

    pub(in crate::ssa) fn structural_variant_tag(
        &self,
        ty: &SsaType,
        variant: lkjscript_ir::VariantId,
    ) -> Result<i64> {
        let item = self
            .structural
            .type_for(ty)
            .ok_or_else(|| Error::msg("structural enum test has no exact type metadata"))?;
        let layout = self
            .structural
            .layouts
            .get(item.layout.index().unwrap_or(usize::MAX))
            .ok_or_else(|| Error::msg("structural enum test has no exact layout metadata"))?;
        let StructuralLayoutKind::Enum { variants, .. } = &layout.kind else {
            return Err(Error::msg("structural enum test has non-enum layout"));
        };
        variants
            .iter()
            .find(|candidate| candidate.variant == variant)
            .map(|candidate| {
                i64::try_from(candidate.physical_tag)
                    .map_err(|_| Error::msg("structural enum tag exceeds i64 observation width"))
            })
            .transpose()?
            .ok_or_else(|| Error::msg("structural enum test has unknown active variant"))
    }
}

fn canonical_structural_route(
    witness: MemoryWitnessId,
    category: crate::memory_plan::MemoryValueCategory,
    storage: StructuralStorage,
    route: u8,
) -> [u8; 32] {
    let mut bytes = b"lkjscript.memory-value-representation\0canonical-platform-contract".to_vec();
    bytes.extend_from_slice(&witness.bytes());
    let category = match category {
        crate::memory_plan::MemoryValueCategory::Owner => 0,
        crate::memory_plan::MemoryValueCategory::View => 1,
        crate::memory_plan::MemoryValueCategory::Destination => 2,
    };
    let storage = match storage {
        StructuralStorage::Inline => 0,
        StructuralStorage::Static => 1,
        StructuralStorage::Stack => 2,
        StructuralStorage::CallerDestination => 3,
        StructuralStorage::UniqueStructural => 4,
        StructuralStorage::OrdinaryRegion => 5,
        StructuralStorage::SealedRegion => 6,
        StructuralStorage::BorrowedView => 7,
        StructuralStorage::ExternalResource => 8,
    };
    let _ = route;
    bytes.extend_from_slice(&[category, storage]);
    lkjscript_core::sha256(&bytes)
}
