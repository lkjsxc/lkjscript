impl FunctionBuilder<'_> {
    #[allow(clippy::too_many_arguments)]
    fn merge_live_branches(
        &mut self,
        result_type: SsaType,
        expression_origin: hir::SourceId,
        incoming_env: BTreeMap<BindingId, ValueId>,
        incoming_slots: BTreeMap<BindingId, u16>,
        mut then_result: BranchResult,
        mut else_result: BranchResult,
        mut then_value: ValueId,
        mut else_value: ValueId,
    ) -> Result<Option<ValueId>> {
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
        let result_owned = is_owned_value(self.structural, &result_type);
        if result_owned {
            then_value = self.normalize_structural_branch_result(
                &mut then_result,
                then_value,
                &result_type,
                &incoming_slots,
                expression_origin,
            )?;
            else_value = self.normalize_structural_branch_result(
                &mut else_result,
                else_value,
                &result_type,
                &incoming_slots,
                expression_origin,
            )?;
        }
        let merge = self.new_block(origin(expression_origin.raw(), self.next_position), false)?;
        let result = self.add_block_parameter(
            merge,
            result_type,
            None,
            origin(expression_origin.raw(), self.next_position),
        )?;
        let mut bindings = Vec::new();
        for binding in incoming_env.keys().copied() {
            match (
                then_result.2.contains_key(&binding),
                else_result.2.contains_key(&binding),
            ) {
                (true, true) => bindings.push(binding),
                (false, false) => {}
                _ => {
                    return Err(Error::msg(
                        "SSA branch ownership environments do not match exactly",
                    ));
                }
            }
        }
        let mut merge_env = BTreeMap::new();
        for binding in &bindings {
            let then_value = then_result
                .2
                .get(binding)
                .copied()
                .ok_or_else(|| Error::msg("SSA merge lost branch binding"))?;
            let ty = self.value_type(then_value)?;
            let owner_place = self.owned_place_for_binding(*binding)?;
            let parameter = self.add_block_parameter(
                merge,
                ty,
                owner_place,
                origin(expression_origin.raw(), self.next_position),
            )?;
            merge_env.insert(*binding, parameter);
        }
        let mut then_residual: Vec<_> = then_result
            .3
            .iter()
            .copied()
            .filter(|value| *value != then_value)
            .collect();
        let mut else_residual: Vec<_> = else_result
            .3
            .iter()
            .copied()
            .filter(|value| *value != else_value)
            .collect();
        let mut shared = 0;
        while shared < then_residual.len() && shared < else_residual.len() {
            if self.value_type(then_residual[shared])?
                != self.value_type(else_residual[shared])?
            {
                break;
            }
            shared += 1;
        }
        self.drop_branch_residuals(
            &mut then_result,
            &then_residual[shared..],
            &incoming_slots,
            expression_origin,
        )?;
        self.drop_branch_residuals(
            &mut else_result,
            &else_residual[shared..],
            &incoming_slots,
            expression_origin,
        )?;
        then_residual.truncate(shared);
        else_residual.truncate(shared);
        let mut merge_unplaced = Vec::with_capacity(then_residual.len());
        for (then_owner, else_owner) in then_residual.iter().zip(&else_residual) {
            let ty = self.value_type(*then_owner)?;
            if self.value_type(*else_owner)? != ty {
                return Err(Error::msg(
                    "SSA branch unplaced owner types do not match exactly",
                ));
            }
            merge_unplaced.push(self.add_block_parameter(
                merge,
                ty,
                None,
                origin(expression_origin.raw(), self.next_position),
            )?);
        }
        let mut then_arguments = edge_arguments(then_value, &bindings, &then_result.2)?;
        then_arguments.extend_from_slice(&then_residual);
        let mut else_arguments = edge_arguments(else_value, &bindings, &else_result.2)?;
        else_arguments.extend_from_slice(&else_residual);
        self.current = then_result.1;
        self.unplaced_owners = then_result.3;
        self.terminate(Terminator::Branch {
            target: merge,
            arguments: then_arguments,
        })?;
        self.current = else_result.1;
        self.unplaced_owners = else_result.3;
        self.terminate(Terminator::Branch {
            target: merge,
            arguments: else_arguments,
        })?;
        self.switch_to(merge)?;
        self.env = merge_env;
        self.slots = incoming_slots;
        self.unplaced_owners = merge_unplaced;
        if result_owned {
            self.unplaced_owners.push(result);
        }
        Ok(Some(result))
    }
}
