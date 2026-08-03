impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_expr(&mut self, expression: &Expr) -> Result<Option<ValueId>> {
        let memory_expression = self.begin_memory_expression()?;
        let previous_placement = self.current_placement;
        self.current_placement = self.cleanup.placement(memory_expression);
        let result = self.lower_expr_inner(expression);
        self.current_placement = previous_placement;
        let result = result?;
        self.finish_memory_expression(memory_expression, expression.origin)?;
        Ok(result)
    }
}
