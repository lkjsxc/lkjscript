use crate::ssa::*;

impl FunctionBuilder<'_> {
    fn loop_target(&self, id: hir::LoopId) -> Result<LoopTarget> {
        self.loops
            .iter()
            .rev()
            .find(|target| target.id == id)
            .cloned()
            .ok_or_else(|| Error::msg("HIR control references an inactive loop"))
    }

    fn loop_environment(&self, target: &LoopTarget) -> Result<Vec<ValueId>> {
        target
            .bindings
            .iter()
            .map(|binding| {
                self.env
                    .get(binding)
                    .copied()
                    .ok_or_else(|| Error::msg("SSA control edge lost loop environment binding"))
            })
            .collect()
    }

    pub(in crate::ssa) fn lower_return(&mut self, value: &Expr) -> Result<Option<ValueId>> {
        let Some(value_id) = self.lower_expr(value)? else {
            return Err(Error::msg("HIR return value is already divergent"));
        };
        self.cleanup_all_places(value.origin)?;
        self.terminate(Terminator::Return(value_id))?;
        Ok(None)
    }

    pub(in crate::ssa) fn lower_break(
        &mut self,
        loop_id: hir::LoopId,
        value: &Expr,
    ) -> Result<Option<ValueId>> {
        let target = self.loop_target(loop_id)?;
        let expression_origin = value.origin;
        let Some(value) = self.lower_expr(value)? else {
            return Err(Error::msg("HIR break value is already divergent"));
        };
        self.cleanup_places_to(target.active_place_bindings.len(), expression_origin)?;
        let mut arguments = Vec::with_capacity(target.bindings.len().saturating_add(1));
        arguments.push(value);
        arguments.extend(self.loop_environment(&target)?);
        self.terminate(Terminator::Branch {
            target: target.exit,
            arguments,
        })?;
        Ok(None)
    }

    pub(in crate::ssa) fn lower_continue(
        &mut self,
        loop_id: hir::LoopId,
        expression_origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let target = self.loop_target(loop_id)?;
        self.cleanup_places_to(target.active_place_bindings.len(), expression_origin)?;
        let arguments = self.loop_environment(&target)?;
        self.terminate(Terminator::Branch {
            target: target.header,
            arguments,
        })?;
        Ok(None)
    }

    pub(in crate::ssa) fn lower_trap(&mut self, value: &Expr) -> Result<Option<ValueId>> {
        let Some(value_id) = self.lower_expr(value)? else {
            return Err(Error::msg("HIR trap value is already divergent"));
        };
        self.cleanup_all_places(value.origin)?;
        self.terminate(Terminator::Trap { value: value_id })?;
        Ok(None)
    }

    pub(in crate::ssa) fn lower_match_unreachable(
        &mut self,
        plan: hir::MatchPlanId,
        origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let message = format!(
            "verified exhaustive match plan {} reached unreachable edge",
            plan.raw()
        );
        let value = self.constant(SsaType::Str, Constant::Str(message), origin)?;
        self.cleanup_all_places(origin)?;
        self.terminate(Terminator::Trap { value })?;
        Ok(None)
    }

    pub(in crate::ssa) fn lower_exit(&mut self, code: &Expr) -> Result<Option<ValueId>> {
        let Some(code_id) = self.lower_expr(code)? else {
            return Err(Error::msg("HIR exit code is already divergent"));
        };
        self.cleanup_all_places(code.origin)?;
        self.terminate(Terminator::Exit { code: code_id })?;
        Ok(None)
    }
}
