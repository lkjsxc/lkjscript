use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_while(
        &mut self,
        condition: &Expr,
        body: &[Expr],
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        let preheader = self
            .current
            .ok_or_else(|| Error::msg("while has no live SSA preheader"))?;
        let incoming_env = self.env.clone();
        let incoming_slots = self.slots.clone();
        let bindings: Vec<BindingId> = incoming_env.keys().copied().collect();
        let header = self.new_block(origin(expression.origin.raw(), self.next_position), true)?;
        let mut header_env = BTreeMap::new();
        for binding in &bindings {
            let incoming = incoming_env
                .get(binding)
                .copied()
                .ok_or_else(|| Error::msg("SSA loop lost incoming binding"))?;
            let ty = self.value_type(incoming)?;
            let owner_place = self.owned_place_for_binding(*binding)?;
            let parameter = self.add_block_parameter(
                header,
                ty,
                owner_place,
                origin(expression.origin.raw(), self.next_position),
            )?;
            header_env.insert(*binding, parameter);
        }
        self.current = Some(preheader);
        self.terminate(Terminator::Branch {
            target: header,
            arguments: bindings
                .iter()
                .map(|binding| {
                    incoming_env
                        .get(binding)
                        .copied()
                        .ok_or_else(|| Error::msg("SSA loop preheader lost binding"))
                })
                .collect::<Result<Vec<_>>>()?,
        })?;
        self.switch_to(header)?;
        self.env = header_env;
        self.slots = incoming_slots.clone();
        let header_frame = self.frame_state();
        self.block_mut(header)?.metadata.frame_state = Some(header_frame);

        let Some(condition_value) = self.lower_expr(condition)? else {
            self.current = None;
            return Ok(None);
        };
        let condition_env = self.env.clone();
        let body_block =
            self.new_block(origin(expression.origin.raw(), self.next_position), false)?;
        let exit_block =
            self.new_block(origin(expression.origin.raw(), self.next_position), false)?;
        let mut body_env = BTreeMap::new();
        let mut exit_env = BTreeMap::new();
        for binding in &bindings {
            let value = condition_env
                .get(binding)
                .copied()
                .ok_or_else(|| Error::msg("SSA loop condition lost binding"))?;
            let ty = self.value_type(value)?;
            let owner_place = self.owned_place_for_binding(*binding)?;
            let body_parameter = self.add_block_parameter(
                body_block,
                ty.clone(),
                owner_place,
                origin(expression.origin.raw(), self.next_position),
            )?;
            body_env.insert(*binding, body_parameter);
            let exit_parameter = self.add_block_parameter(
                exit_block,
                ty,
                owner_place,
                origin(expression.origin.raw(), self.next_position),
            )?;
            exit_env.insert(*binding, exit_parameter);
        }
        let condition_arguments: Vec<ValueId> = bindings
            .iter()
            .map(|binding| {
                condition_env
                    .get(binding)
                    .copied()
                    .ok_or_else(|| Error::msg("SSA loop edge lost binding"))
            })
            .collect::<Result<Vec<_>>>()?;
        self.terminate(Terminator::ConditionalBranch {
            condition: condition_value,
            true_target: body_block,
            true_arguments: condition_arguments.clone(),
            false_target: exit_block,
            false_arguments: condition_arguments,
        })?;

        self.switch_to(body_block)?;
        self.env = body_env;
        self.slots = incoming_slots.clone();
        let body_result = self.lower_sequence(body, expression.origin)?;
        let mut has_backedge = false;
        if body_result.is_some() {
            let arguments = bindings
                .iter()
                .map(|binding| {
                    self.env
                        .get(binding)
                        .copied()
                        .ok_or_else(|| Error::msg("SSA loop body lost binding"))
                })
                .collect::<Result<Vec<_>>>()?;
            self.terminate(Terminator::Branch {
                target: header,
                arguments,
            })?;
            has_backedge = true;
        }
        if !has_backedge {
            self.block_mut(header)?.metadata.loop_header = false;
            self.block_mut(header)?.metadata.frame_state = None;
        }

        self.switch_to(exit_block)?;
        self.env = exit_env;
        self.slots = incoming_slots;
        self.constant(SsaType::Unit, Constant::Unit, expression.origin)
            .map(Some)
    }
}
