use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_sequence(
        &mut self,
        expressions: &[Expr],
        sequence_origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let mut result = None;
        for expression in expressions {
            result = self.lower_expr(expression)?;
            if result.is_none() {
                return Ok(None);
            }
        }
        if let Some(result) = result {
            Ok(Some(result))
        } else {
            self.constant(SsaType::Unit, Constant::Unit, sequence_origin)
                .map(Some)
        }
    }

    pub(in crate::ssa) fn lower_let(
        &mut self,
        bindings: &[LocalDefinition],
        body: &Expr,
    ) -> Result<Option<ValueId>> {
        let mut previous = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let Some(value) = self.lower_expr(&binding.value)? else {
                for (binding, previous_value, previous_slot) in previous.into_iter().rev() {
                    restore(&mut self.env, binding, previous_value);
                    restore(&mut self.slots, binding, previous_slot);
                }
                return Ok(None);
            };
            let place_ty = lower_type(&binding.value.ty, self.product_ids)?;
            self.register_place(binding.place, binding.binding, place_ty)?;
            self.initialize_owned_place(binding.binding, value, binding.value.origin)?;
            previous.push((
                binding.binding,
                self.env.insert(binding.binding, value),
                self.slots.insert(binding.binding, u16::from(binding.slot)),
            ));
        }
        let result = self.lower_expr(body)?;
        if result.is_some() {
            for (binding, _, _) in previous.iter().rev() {
                self.end_owned_place(*binding, body.origin)?;
            }
        }
        for (binding, previous_value, previous_slot) in previous.into_iter().rev() {
            restore(&mut self.env, binding, previous_value);
            restore(&mut self.slots, binding, previous_slot);
        }
        Ok(result)
    }
}
