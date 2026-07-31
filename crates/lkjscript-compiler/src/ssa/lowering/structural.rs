use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn publish_structural_source(
        &mut self,
        ty: SsaType,
        source: ValueId,
        expression_origin: hir::SourceId,
    ) -> Result<ValueId> {
        let Some(representation) = self
            .structural
            .representation(&ty, StructuralValueCategory::Owner)
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
            .structural
            .representation(&ty, StructuralValueCategory::Destination)
            .ok_or_else(|| {
                Error::msg("selected structural aggregate has no destination representation")
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
            let field = u16::try_from(index)
                .map_err(|_| Error::msg("structural destination field exceeds u16"))?;
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
        self.structural.representation(ty, category).ok_or_else(|| {
            Error::msg(format!(
                "selected structural type {ty:?} has no {category:?} representation"
            ))
        })
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
            .map(|candidate| i64::from(candidate.physical_tag))
            .ok_or_else(|| Error::msg("structural enum test has unknown active variant"))
    }
}
