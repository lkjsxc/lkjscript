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
        let Some(value) = self.lower_expr(value)? else {
            return Err(Error::msg("HIR return value is already divergent"));
        };
        self.terminate(Terminator::Return(value))?;
        Ok(None)
    }

    pub(in crate::ssa) fn lower_break(
        &mut self,
        loop_id: hir::LoopId,
        value: &Expr,
    ) -> Result<Option<ValueId>> {
        let target = self.loop_target(loop_id)?;
        let Some(value) = self.lower_expr(value)? else {
            return Err(Error::msg("HIR break value is already divergent"));
        };
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
    ) -> Result<Option<ValueId>> {
        let target = self.loop_target(loop_id)?;
        let arguments = self.loop_environment(&target)?;
        self.terminate(Terminator::Branch {
            target: target.header,
            arguments,
        })?;
        Ok(None)
    }

    pub(in crate::ssa) fn lower_trap(&mut self, value: &Expr) -> Result<Option<ValueId>> {
        let Some(value) = self.lower_expr(value)? else {
            return Err(Error::msg("HIR trap value is already divergent"));
        };
        self.terminate(Terminator::Trap { value })?;
        Ok(None)
    }

    pub(in crate::ssa) fn lower_exit(&mut self, code: &Expr) -> Result<Option<ValueId>> {
        let Some(code) = self.lower_expr(code)? else {
            return Err(Error::msg("HIR exit code is already divergent"));
        };
        self.terminate(Terminator::Exit { code })?;
        Ok(None)
    }
}
