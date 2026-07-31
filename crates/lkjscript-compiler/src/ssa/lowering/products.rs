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
        if self.structural.is_owned(&ty) {
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
                "structural product field projection requires explicit owner lowering",
            ));
        }
        let value = self.append(
            ty,
            InstructionKind::ProductField {
                product,
                field,
                value,
            },
            EffectSet::READS_MEMORY,
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
        if self.structural.is_owned(&ty) {
            return Err(Error::msg(
                "structural product update is unavailable without whole-owner reconstruction",
            ));
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
}
