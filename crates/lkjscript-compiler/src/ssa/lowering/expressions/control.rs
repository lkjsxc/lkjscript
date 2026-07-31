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
        let (then_env, then_unplaced, incoming_arguments) = self.add_edge_state_parameters(
            then_block,
            &incoming_env,
            &incoming_unplaced,
            branch_origin,
        )?;
        let (else_env, else_unplaced, else_arguments) = self.add_edge_state_parameters(
            else_block,
            &incoming_env,
            &incoming_unplaced,
            branch_origin,
        )?;
        if incoming_arguments != else_arguments {
            return Err(Error::msg("SSA conditional edge schemas diverged"));
        }
        self.terminate(Terminator::ConditionalBranch {
            condition: condition_value,
            true_target: then_block,
            true_arguments: incoming_arguments.clone(),
            false_target: else_block,
            false_arguments: incoming_arguments,
        })?;

        self.switch_to(then_block)?;
        self.unplaced_owners = then_unplaced;
        self.env = then_env;
        self.slots = incoming_slots.clone();
        let then_value = self.lower_expr(then_branch)?;
        let then_end = self.current;
        let then_env = self.env.clone();
        let then_active_places = self.active_place_bindings.clone();
        let then_unplaced = self.unplaced_owners.clone();

        self.switch_to(else_block)?;
        self.active_place_bindings = incoming_active_places.clone();
        self.unplaced_owners = else_unplaced;
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
        self.merge_branches(
            result_type,
            expression.origin,
            incoming_env,
            incoming_slots,
            incoming_unplaced,
            (then_value, then_end, then_env, then_unplaced),
            (else_value, else_end, else_env, else_unplaced),
        )
    }
}
