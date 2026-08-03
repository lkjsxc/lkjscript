use crate::ssa::*;

struct OpenLoop {
    target: LoopTarget,
    header_env: BTreeMap<BindingId, ValueId>,
    exit_env: BTreeMap<BindingId, ValueId>,
}

impl FunctionBuilder<'_> {
    fn open_loop(
        &mut self,
        loop_id: hir::LoopId,
        result_type: SsaType,
        expression: &Expr,
    ) -> Result<OpenLoop> {
        let preheader = self
            .current
            .ok_or_else(|| Error::msg("loop has no live SSA preheader"))?;
        if !self.unplaced_owners.is_empty() {
            return Err(Error::msg(
                "structural temporary ownership cannot cross a loop boundary",
            ));
        }
        let incoming = self.env.clone();
        let bindings: Vec<_> = incoming.keys().copied().collect();
        let block_origin = origin(expression.origin.raw(), self.next_position);
        let header = self.new_block(block_origin, true)?;
        let exit = self.new_block(block_origin, false)?;
        let header_env = self.add_environment_parameters(header, &incoming, block_origin)?;
        let _result = self.add_block_parameter(exit, result_type, None, block_origin)?;
        let exit_env = self.add_environment_parameters(exit, &incoming, block_origin)?;
        self.current = Some(preheader);
        self.terminate(Terminator::Branch {
            target: header,
            arguments: Self::environment_arguments(&incoming),
        })?;
        Ok(OpenLoop {
            target: LoopTarget {
                id: loop_id,
                header,
                exit,
                bindings,
                active_place_bindings: self.active_place_bindings.clone(),
            },
            header_env,
            exit_env,
        })
    }

    fn close_loop(
        &mut self,
        target: LoopTarget,
        exit_env: BTreeMap<BindingId, ValueId>,
        incoming_slots: BTreeMap<BindingId, u16>,
    ) -> Result<Option<ValueId>> {
        let result = self
            .block_mut(target.exit)?
            .parameters
            .first()
            .map(|parameter| parameter.id)
            .ok_or_else(|| Error::msg("typed loop exit lost result parameter"))?;
        self.switch_to(target.exit)?;
        self.active_place_bindings = target.active_place_bindings.clone();
        self.env = exit_env;
        self.slots = incoming_slots;
        self.unplaced_owners = if is_owned_value(self.structural, &self.value_type(result)?) {
            vec![result]
        } else {
            Vec::new()
        };
        Ok(Some(result))
    }

    pub(in crate::ssa) fn lower_loop(
        &mut self,
        loop_id: hir::LoopId,
        result_type: &Type,
        body: &[Expr],
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        let slots = self.slots.clone();
        let result_type = lower_type(result_type, self.product_ids)?;
        let OpenLoop {
            target,
            header_env,
            exit_env,
        } = self.open_loop(loop_id, result_type, expression)?;
        self.switch_to(target.header)?;
        self.env = header_env;
        self.block_mut(target.header)?.metadata.frame_state = Some(self.frame_state());
        self.loops.push(target.clone());
        let body_result = self.lower_sequence(body, expression.origin)?;
        let _active = self.loops.pop();
        if body_result.is_some() {
            let arguments = self.loop_environment_for(&target)?;
            self.terminate(Terminator::Branch {
                target: target.header,
                arguments,
            })?;
        }
        self.clear_noncyclic_header(&target)?;
        self.close_loop(target, exit_env, slots)
    }

    pub(in crate::ssa) fn lower_while(
        &mut self,
        loop_id: hir::LoopId,
        condition: &Expr,
        body: &[Expr],
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        let slots = self.slots.clone();
        let OpenLoop {
            target,
            header_env,
            exit_env,
        } = self.open_loop(loop_id, SsaType::Unit, expression)?;
        self.switch_to(target.header)?;
        self.env = header_env;
        self.slots = slots.clone();
        self.block_mut(target.header)?.metadata.frame_state = Some(self.frame_state());
        self.loops.push(target.clone());
        let Some(condition_value) = self.lower_expr(condition)? else {
            let _active = self.loops.pop();
            self.clear_noncyclic_header(&target)?;
            return self.close_loop(target, exit_env, slots);
        };
        self.drop_abandoned_structural_owners(condition_value, condition.origin)?;
        let condition_env = self.env.clone();
        let body_block =
            self.new_block(origin(expression.origin.raw(), self.next_position), false)?;
        let body_env = self.add_environment_parameters(
            body_block,
            &condition_env,
            origin(expression.origin.raw(), self.next_position),
        )?;
        let unit = self.constant(SsaType::Unit, Constant::Unit, expression.origin)?;
        let mut exit_arguments = vec![unit];
        exit_arguments.extend(Self::environment_arguments(&condition_env));
        self.terminate(Terminator::ConditionalBranch {
            condition: condition_value,
            true_target: body_block,
            true_arguments: Self::environment_arguments(&condition_env),
            false_target: target.exit,
            false_arguments: exit_arguments,
        })?;
        self.switch_to(body_block)?;
        self.env = body_env;
        self.slots = slots.clone();
        let body_result = self.lower_sequence(body, expression.origin)?;
        let _active = self.loops.pop();
        if body_result.is_some() {
            let arguments = self.loop_environment_for(&target)?;
            self.terminate(Terminator::Branch {
                target: target.header,
                arguments,
            })?;
        }
        self.clear_noncyclic_header(&target)?;
        self.close_loop(target, exit_env, slots)
    }

    fn clear_noncyclic_header(&mut self, target: &LoopTarget) -> Result<()> {
        let incoming = self
            .blocks
            .iter()
            .filter(|block| match block.terminator.as_ref() {
                Some(Terminator::Branch { target: edge, .. }) => *edge == target.header,
                Some(Terminator::ConditionalBranch {
                    true_target,
                    false_target,
                    ..
                }) => *true_target == target.header || *false_target == target.header,
                _ => false,
            })
            .count();
        if incoming <= 1 {
            let header = self.block_mut(target.header)?;
            header.metadata.loop_header = false;
            header.metadata.frame_state = None;
        }
        Ok(())
    }

    fn loop_environment_for(&self, target: &LoopTarget) -> Result<Vec<ValueId>> {
        target
            .bindings
            .iter()
            .map(|binding| {
                self.env
                    .get(binding)
                    .copied()
                    .ok_or_else(|| Error::msg("SSA loop body lost environment binding"))
            })
            .collect()
    }
}
