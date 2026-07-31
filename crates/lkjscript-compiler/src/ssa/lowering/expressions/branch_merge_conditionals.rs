impl FunctionBuilder<'_> {
    fn normalize_conditional_branch_bindings(
        &mut self,
        incoming_env: &BTreeMap<BindingId, ValueId>,
        then_result: &mut BranchResult,
        else_result: &mut BranchResult,
        then_value: ValueId,
        else_value: ValueId,
        expression_origin: hir::SourceId,
    ) -> Result<()> {
        let conditional: Vec<_> = incoming_env
            .keys()
            .copied()
            .filter(|binding| {
                then_result.2.contains_key(binding) != else_result.2.contains_key(binding)
            })
            .collect();
        for binding in conditional {
            if then_result.2.contains_key(&binding) {
                self.verify_conditional_absent_branch(binding, else_result.1, else_value)?;
                self.current = else_result.1;
                self.env = else_result.2.clone();
                self.unplaced_owners = else_result.3.clone();
                self.end_conditional_branch_place(binding, expression_origin)?;
                else_result.1 = self.current;
                else_result.2 = self.env.clone();

                self.current = then_result.1;
                self.env = then_result.2.clone();
                self.unplaced_owners = then_result.3.clone();
                self.drop_conditional_branch_owner(binding, expression_origin)?;
                self.end_conditional_branch_place(binding, expression_origin)?;
                then_result.1 = self.current;
                then_result.2 = self.env.clone();
            } else {
                self.verify_conditional_absent_branch(binding, then_result.1, then_value)?;
                self.current = then_result.1;
                self.env = then_result.2.clone();
                self.unplaced_owners = then_result.3.clone();
                self.end_conditional_branch_place(binding, expression_origin)?;
                then_result.1 = self.current;
                then_result.2 = self.env.clone();

                self.current = else_result.1;
                self.env = else_result.2.clone();
                self.unplaced_owners = else_result.3.clone();
                self.drop_conditional_branch_owner(binding, expression_origin)?;
                self.end_conditional_branch_place(binding, expression_origin)?;
                else_result.1 = self.current;
                else_result.2 = self.env.clone();
            }
        }
        Ok(())
    }
}
