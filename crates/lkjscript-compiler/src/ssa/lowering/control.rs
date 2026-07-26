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

        self.switch_to(else_block)?;
        self.env = else_env;
        self.slots = incoming_slots.clone();
        let else_value = self.lower_expr(else_branch)?;
        let else_end = self.current;
        let else_env = self.env.clone();

        let result_type = if expression.ty == Type::Never {
            SsaType::Unit
        } else {
            lower_type(&expression.ty, self.product_ids)?
        };
        self.merge_branches(
            result_type,
            expression.origin,
            incoming_env,
            incoming_slots,
            (then_value, then_end, then_env),
            (else_value, else_end, else_env),
        )
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

        self.switch_to(skip_right)?;
        self.env = skipped_entry_env;
        self.slots = incoming_slots.clone();
        let skipped_value =
            self.constant(SsaType::Bool, Constant::Bool(skipped), expression.origin)?;
        let skipped_end = self.current;
        let skipped_env = self.env.clone();

        let (then_result, else_result) = if operation == Operation::And {
            (
                (right_value, right_end, right_env),
                (Some(skipped_value), skipped_end, skipped_env),
            )
        } else {
            (
                (Some(skipped_value), skipped_end, skipped_env),
                (right_value, right_end, right_env),
            )
        };
        self.merge_branches(
            SsaType::Bool,
            expression.origin,
            incoming_env,
            incoming_slots,
            then_result,
            else_result,
        )
    }
}
