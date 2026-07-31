impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_expr(&mut self, expression: &Expr) -> Result<Option<ValueId>> {
        let memory_expression = self.begin_memory_expression()?;
        let result = self.lower_expr_inner(expression)?;
        self.finish_memory_expression(memory_expression, expression.origin)?;
        Ok(result)
    }
}
