use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_move(
        &mut self,
        place: hir::PlaceId,
        binding: hir::BindingRef,
        ty: SsaType,
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        let Some(value) = self.lower_load(binding, expression)? else {
            return Ok(None);
        };
        let moved = self.append(
            ty,
            InstructionKind::Move {
                place: SsaPlaceId::new(place.raw()),
                value,
            },
            EffectSet::PURE,
            expression.origin,
        )?;
        self.env.remove(&binding.binding);
        Ok(Some(moved))
    }

    pub(in crate::ssa) fn lower_borrow(
        &mut self,
        place: hir::PlaceId,
        loan: hir::LoanId,
        kind: hir::BorrowKind,
        binding: hir::BindingRef,
        ty: SsaType,
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        let Some(value) = self.lower_load(binding, expression)? else {
            return Ok(None);
        };
        let value = self.append(
            ty,
            InstructionKind::Borrow {
                place: SsaPlaceId::new(place.raw()),
                loan: SsaLoanId::new(loan.raw()),
                kind: match kind {
                    hir::BorrowKind::Shared => SsaBorrowKind::Shared,
                    hir::BorrowKind::Mutable => SsaBorrowKind::Mutable,
                },
                value,
            },
            EffectSet::PURE,
            expression.origin,
        )?;
        Ok(Some(value))
    }
}
