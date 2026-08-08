impl FunctionBuilder<'_> {
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
        let branch_origin = origin(expression.origin, self.next_position);
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
        let (right_entry_env, right_entry_unplaced, incoming_arguments) = self
            .add_edge_state_parameters(
                evaluate_right,
                &incoming_env,
                &incoming_unplaced,
                branch_origin,
            )?;
        let (skipped_entry_env, skipped_entry_unplaced, skipped_arguments) = self
            .add_edge_state_parameters(
                skip_right,
                &incoming_env,
                &incoming_unplaced,
                branch_origin,
            )?;
        if incoming_arguments != skipped_arguments {
            return Err(Error::msg("SSA short-circuit edge schemas diverged"));
        }
        self.terminate(Terminator::ConditionalBranch {
            condition: left,
            true_target,
            true_arguments: incoming_arguments.clone(),
            false_target,
            false_arguments: incoming_arguments,
        })?;

        self.switch_to(evaluate_right)?;
        self.unplaced_owners = right_entry_unplaced;
        self.env = right_entry_env;
        self.slots = incoming_slots.clone();
        let right_value = self.lower_expr(right)?;
        let right_end = self.current;
        let right_env = self.env.clone();
        let right_active_places = self.active_place_bindings.clone();
        let right_unplaced = self.unplaced_owners.clone();

        self.switch_to(skip_right)?;
        self.active_place_bindings = incoming_active_places.clone();
        self.unplaced_owners = skipped_entry_unplaced;
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
