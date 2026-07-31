#[derive(Clone, Copy)]
pub(in crate::ssa) struct StructuralOwnerPlace {
    pub(in crate::ssa) place: SsaPlaceId,
    pub(in crate::ssa) binding: BindingId,
}

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn structural_owner_place(
        &mut self,
        expression: &Expr,
        value: ValueId,
        ty: &SsaType,
        expression_origin: hir::SourceId,
    ) -> Result<StructuralOwnerPlace> {
        if let ExprKind::Load(reference) = expression.kind {
            let place = self
                .owned_place_for_binding(reference.binding)?
                .ok_or_else(|| Error::msg("structural owner load has no exact place"))?;
            if self.env.get(&reference.binding) != Some(&value) {
                return Err(Error::msg("structural owner load is stale at projection"));
            }
            return Ok(StructuralOwnerPlace {
                place,
                binding: reference.binding,
            });
        }
        if let ExprKind::Move { place, binding } = expression.kind {
            return Ok(StructuralOwnerPlace {
                place: SsaPlaceId::new(place.raw()),
                binding: binding.binding,
            });
        }
        self.synthetic_structural_owner_place(value, ty, expression_origin)
    }
}
