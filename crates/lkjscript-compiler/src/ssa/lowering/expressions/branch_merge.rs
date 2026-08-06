impl FunctionBuilder<'_> {
    #[allow(clippy::too_many_arguments)]
    fn merge_live_branches(
        &mut self,
        result_type: SsaType,
        expression_origin: hir::SourceId,
        incoming_env: BTreeMap<BindingId, ValueId>,
        incoming_slots: BTreeMap<BindingId, u64>,
        mut then_result: BranchResult,
        mut else_result: BranchResult,
        mut then_value: ValueId,
        mut else_value: ValueId,
    ) -> Result<Option<ValueId>> {
        self.normalize_conditional_branch_bindings(
            &incoming_env,
            &mut then_result,
            &mut else_result,
            then_value,
            else_value,
            expression_origin,
        )?;
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
        let mut owner_parameters = BTreeMap::new();
        let mut argument_bindings = Vec::new();
        for binding in &bindings {
            let then_value = then_result
                .2
                .get(binding)
                .copied()
                .ok_or_else(|| Error::msg("SSA merge lost then-branch binding"))?;
            let else_value = else_result
                .2
                .get(binding)
                .copied()
                .ok_or_else(|| Error::msg("SSA merge lost else-branch binding"))?;
            let owner_place = self.owned_place_for_binding(*binding)?;
            let key = (then_value, else_value);
            if let Some((parameter, existing_place)) = owner_parameters.get(&key).copied() {
                if owner_place != existing_place {
                    return Err(Error::msg(
                        "SSA merge aliases one owner through distinct ownership places",
                    ));
                }
                merge_env.insert(*binding, parameter);
                continue;
            }
            let ty = self.value_type(then_value)?;
            if self.value_type(else_value)? != ty {
                return Err(Error::msg("SSA merge binding types do not match exactly"));
            }
            let parameter = self.add_block_parameter(
                merge,
                ty,
                owner_place,
                origin(expression_origin.raw(), self.next_position),
            )?;
            owner_parameters.insert(key, (parameter, owner_place));
            argument_bindings.push(*binding);
            merge_env.insert(*binding, parameter);
        }
        let mut then_residual = Self::unbound_residual(&then_result, then_value);
        let mut else_residual = Self::unbound_residual(&else_result, else_value);
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
        let mut then_arguments =
            edge_arguments(then_value, &argument_bindings, &then_result.2)?;
        then_arguments.extend_from_slice(&then_residual);
        let mut else_arguments =
            edge_arguments(else_value, &argument_bindings, &else_result.2)?;
        else_arguments.extend_from_slice(&else_residual);
        self.current = then_result.1;
        self.env = then_result.2.clone();
        self.slots = incoming_slots.clone();
        self.unplaced_owners = then_result.3.clone();
        self.terminate(Terminator::Branch {
            target: merge,
            arguments: then_arguments,
        })?;
        self.current = else_result.1;
        self.env = else_result.2.clone();
        self.slots = incoming_slots.clone();
        self.unplaced_owners = else_result.3.clone();
        self.terminate(Terminator::Branch {
            target: merge,
            arguments: else_arguments,
        })?;
        self.switch_to(merge)?;
        let mut bound_unplaced = self.merged_bound_unplaced(
            &bindings,
            &merge_env,
            &then_result,
            &else_result,
        )?;
        bound_unplaced.extend(merge_unplaced);
        self.env = merge_env;
        self.slots = incoming_slots;
        self.unplaced_owners = bound_unplaced;
        if result_owned {
            self.unplaced_owners.push(result);
        }
        Ok(Some(result))
    }
}
