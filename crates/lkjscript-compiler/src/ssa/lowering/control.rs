use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_if(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: &Expr,
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        let Some(condition_value) = self.lower_expr(condition)? else {
            return Ok(None);
        };
        let branch_origin = origin(expression.origin.raw(), self.next_position);
        let then_block = self.new_block(branch_origin, false)?;
        let else_block = self.new_block(branch_origin, false)?;
        let incoming_env = self.env.clone();
        let incoming_slots = self.slots.clone();
        let incoming_active_places = self.active_place_bindings.clone();
        let incoming_unplaced = self.unplaced_owners.clone();
        let then_env = self.add_environment_parameters(then_block, &incoming_env, branch_origin)?;
        let else_env = self.add_environment_parameters(else_block, &incoming_env, branch_origin)?;
        let incoming_arguments = Self::environment_arguments(&incoming_env);
        self.terminate(Terminator::ConditionalBranch {
            condition: condition_value,
            true_target: then_block,
            true_arguments: incoming_arguments.clone(),
            false_target: else_block,
            false_arguments: incoming_arguments,
        })?;

        self.switch_to(then_block)?;
        self.env = then_env;
        self.slots = incoming_slots.clone();
        let then_value = self.lower_expr(then_branch)?;
        let then_end = self.current;
        let then_env = self.env.clone();
        let then_active_places = self.active_place_bindings.clone();
        let then_unplaced = self.unplaced_owners.clone();

        self.switch_to(else_block)?;
        self.active_place_bindings = incoming_active_places.clone();
        self.unplaced_owners = incoming_unplaced.clone();
        self.env = else_env;
        self.slots = incoming_slots.clone();
        let else_value = self.lower_expr(else_branch)?;
        let else_end = self.current;
        let else_env = self.env.clone();
        let else_active_places = self.active_place_bindings.clone();
        let else_unplaced = self.unplaced_owners.clone();
        let merged_active_places = match (then_value.is_some(), else_value.is_some()) {
            (true, true) if then_active_places == else_active_places => then_active_places,
            (true, true) => {
                return Err(Error::msg("SSA branch cleanup states do not match exactly"));
            }
            (true, false) => then_active_places,
            (false, true) => else_active_places,
            (false, false) => incoming_active_places,
        };

        self.active_place_bindings = merged_active_places;
        let result_type = if expression.ty == Type::Never {
            SsaType::Unit
        } else {
            lower_type(&expression.ty, self.product_ids)?
        };
        let result = self.merge_branches(
            result_type,
            expression.origin,
            incoming_env,
            incoming_slots,
            incoming_unplaced,
            (then_value, then_end, then_env, then_unplaced),
            (else_value, else_end, else_env, else_unplaced),
        )?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::ssa) fn lower_short_circuit(
        &mut self,
        operation: Operation,
        arguments: &[Expr],
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        let [left, right] = arguments else {
            return Err(Error::msg(
                "resolved short-circuit operation must have two arguments",
            ));
        };
        let Some(left) = self.lower_expr(left)? else {
            return Ok(None);
        };
        let branch_origin = origin(expression.origin.raw(), self.next_position);
        let evaluate_right = self.new_block(branch_origin, false)?;
        let skip_right = self.new_block(branch_origin, false)?;
        let (true_target, false_target, skipped) = if operation == Operation::And {
            (evaluate_right, skip_right, false)
        } else {
            (skip_right, evaluate_right, true)
        };
        let incoming_env = self.env.clone();
        let incoming_slots = self.slots.clone();
        let incoming_active_places = self.active_place_bindings.clone();
        let incoming_unplaced = self.unplaced_owners.clone();
        let right_entry_env =
            self.add_environment_parameters(evaluate_right, &incoming_env, branch_origin)?;
        let skipped_entry_env =
            self.add_environment_parameters(skip_right, &incoming_env, branch_origin)?;
        let incoming_arguments = Self::environment_arguments(&incoming_env);
        self.terminate(Terminator::ConditionalBranch {
            condition: left,
            true_target,
            true_arguments: incoming_arguments.clone(),
            false_target,
            false_arguments: incoming_arguments,
        })?;

        self.switch_to(evaluate_right)?;
        self.env = right_entry_env;
        self.slots = incoming_slots.clone();
        let right_value = self.lower_expr(right)?;
        let right_end = self.current;
        let right_env = self.env.clone();
        let right_active_places = self.active_place_bindings.clone();
        let right_unplaced = self.unplaced_owners.clone();

        self.switch_to(skip_right)?;
        self.active_place_bindings = incoming_active_places.clone();
        self.unplaced_owners = incoming_unplaced.clone();
        self.env = skipped_entry_env;
        self.slots = incoming_slots.clone();
        let skipped_value =
            self.constant(SsaType::Bool, Constant::Bool(skipped), expression.origin)?;
        let skipped_end = self.current;
        let skipped_env = self.env.clone();
        let skipped_active_places = self.active_place_bindings.clone();
        let skipped_unplaced = self.unplaced_owners.clone();

        let (then_result, else_result) = if operation == Operation::And {
            (
                (right_value, right_end, right_env, right_unplaced),
                (
                    Some(skipped_value),
                    skipped_end,
                    skipped_env,
                    skipped_unplaced,
                ),
            )
        } else {
            (
                (
                    Some(skipped_value),
                    skipped_end,
                    skipped_env,
                    skipped_unplaced,
                ),
                (right_value, right_end, right_env, right_unplaced),
            )
        };
        let result = self.merge_branches(
            SsaType::Bool,
            expression.origin,
            incoming_env,
            incoming_slots,
            incoming_unplaced,
            then_result,
            else_result,
        )?;
        self.active_place_bindings = if right_value.is_some() {
            if right_active_places != skipped_active_places {
                return Err(Error::msg(
                    "SSA short-circuit cleanup states do not match exactly",
                ));
            }
            right_active_places
        } else {
            skipped_active_places
        };
        Ok(result)
    }
}
