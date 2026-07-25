use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_mutable_local(
        &mut self,
        binding: BindingId,
        place: hir::PlaceId,
        slot: u8,
        initial: &Expr,
        body: &Expr,
        origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let place_ty = lower_type(&initial.ty, self.product_ids)?;
        let Some(initial) = self.lower_expr(initial)? else {
            return Ok(None);
        };
        self.register_place(place, binding, place_ty)?;
        self.initialize_owned_place(binding, initial, origin)?;
        let previous_value = self.env.insert(binding, initial);
        let previous_slot = self.slots.insert(binding, u16::from(slot));
        let result = self.lower_expr(body)?;
        if result.is_some() {
            self.end_owned_place(binding, origin)?;
        }
        restore(&mut self.env, binding, previous_value);
        restore(&mut self.slots, binding, previous_slot);
        Ok(result)
    }

    pub(in crate::ssa) fn lower_set_local(
        &mut self,
        target: BindingId,
        slot: u8,
        value: &Expr,
        origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let Some(value) = self.lower_expr(value)? else {
            return Ok(None);
        };
        let owned_place = self.owned_place_for_binding(target)?;
        if owned_place.is_none() && !self.env.contains_key(&target) {
            return Err(Error::msg(format!(
                "HIR set target {} is not in SSA environment",
                target.raw()
            )));
        }
        if owned_place.is_some() {
            self.initialize_owned_place(target, value, origin)?;
        }
        self.env.insert(target, value);
        self.slots.insert(target, u16::from(slot));
        self.constant(SsaType::Unit, Constant::Unit, origin)
            .map(Some)
    }
}
