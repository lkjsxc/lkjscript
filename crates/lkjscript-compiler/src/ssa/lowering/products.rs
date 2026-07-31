use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_product_value(
        &mut self,
        product: ProductId,
        fields: &[Expr],
        origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let Some(fields) = self.lower_arguments(fields)? else {
            return Ok(None);
        };
        let ty = SsaType::Product(product);
        if self.structural.type_for(&ty).is_some() {
            return self.construct_structural_aggregate(ty, None, fields, origin);
        }
        let value = self.append(
            ty,
            InstructionKind::ProductValue { product, fields },
            EffectSet::ALLOCATES,
            origin,
        )?;
        Ok(Some(value))
    }

    pub(in crate::ssa) fn lower_product_field(
        &mut self,
        product: ProductId,
        field: u8,
        value: &Expr,
        ty: SsaType,
        origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let Some(value) = self.lower_expr(value)? else {
            return Ok(None);
        };
        let owner_ty = SsaType::Product(product);
        if self.structural.is_owned(&owner_ty) {
            return Err(Error::msg(
                "owned structural product field projection requires explicit owner lowering",
            ));
        }
        let effects = if self.structural.type_for(&owner_ty).is_some() {
            EffectSet::READS_MEMORY.union(EffectSet::ALLOCATES)
        } else {
            EffectSet::READS_MEMORY
        };
        let value = self.append(
            ty,
            InstructionKind::ProductField {
                product,
                field,
                value,
            },
            effects,
            origin,
        )?;
        Ok(Some(value))
    }

    pub(in crate::ssa) fn lower_product_update(
        &mut self,
        product: ProductId,
        field: u8,
        value: &Expr,
        replacement: &Expr,
        origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let Some(value) = self.lower_expr(value)? else {
            return Ok(None);
        };
        let Some(replacement) = self.lower_expr(replacement)? else {
            return Ok(None);
        };
        let ty = SsaType::Product(product);
        if let Some(item) = self.structural.type_for(&ty) {
            if item.mode != StructuralTypeMode::Copy {
                return Err(Error::msg(
                    "owned structural product update requires explicit owner lowering",
                ));
            }
            let fields = self.structural_product_fields(item)?.to_vec();
            if usize::from(field) >= fields.len() {
                return Err(Error::msg(
                    "structural product update field is out of range",
                ));
            }
            let mut rebuilt = Vec::new();
            rebuilt
                .try_reserve_exact(fields.len())
                .map_err(|_| Error::msg("structural product update allocation failed"))?;
            for (index, field_ty) in fields.into_iter().enumerate() {
                if index == usize::from(field) {
                    rebuilt.push(replacement);
                } else {
                    rebuilt.push(self.append(
                        field_ty,
                        InstructionKind::ProductField {
                            product,
                            field:
                                u8::try_from(index).map_err(|_| {
                                    Error::msg("structural product field exceeds u8")
                                })?,
                            value,
                        },
                        EffectSet::READS_MEMORY.union(EffectSet::ALLOCATES),
                        origin,
                    )?);
                }
            }
            return self.construct_structural_aggregate(ty, None, rebuilt, origin);
        }
        let value = self.append(
            ty,
            InstructionKind::WithProductField {
                product,
                field,
                value,
                replacement,
            },
            EffectSet::READS_MEMORY.union(EffectSet::ALLOCATES),
            origin,
        )?;
        Ok(Some(value))
    }

    fn structural_product_fields(&self, item: &StructuralTypeMetadata) -> Result<&[SsaType]> {
        let layout = self
            .structural
            .layouts
            .get(item.layout.index().unwrap_or(usize::MAX))
            .ok_or_else(|| Error::msg("structural product layout is missing"))?;
        let StructuralLayoutKind::Product { fields, .. } = &layout.kind else {
            return Err(Error::msg("structural product type has non-product layout"));
        };
        Ok(fields)
    }
}
