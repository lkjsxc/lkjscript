use crate::ssa::*;

impl FunctionBuilder<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::ssa) fn merge_branches(
        &mut self,
        result_type: SsaType,
        expression_origin: hir::SourceId,
        incoming_env: BTreeMap<BindingId, ValueId>,
        incoming_slots: BTreeMap<BindingId, u16>,
        incoming_unplaced: Vec<ValueId>,
        mut then_result: (
            Option<ValueId>,
            Option<BlockId>,
            BTreeMap<BindingId, ValueId>,
            Vec<ValueId>,
        ),
        mut else_result: (
            Option<ValueId>,
            Option<BlockId>,
            BTreeMap<BindingId, ValueId>,
            Vec<ValueId>,
        ),
    ) -> Result<Option<ValueId>> {
        match (then_result.0, else_result.0) {
            (None, None) => {
                self.current = None;
                self.env = incoming_env;
                self.slots = incoming_slots;
                self.unplaced_owners = incoming_unplaced;
                Ok(None)
            }
            (Some(value), None) => {
                self.current = then_result.1;
                self.env = then_result.2;
                self.slots = incoming_slots;
                self.unplaced_owners = then_result.3;
                Ok(Some(value))
            }
            (None, Some(value)) => {
                self.current = else_result.1;
                self.env = else_result.2;
                self.slots = incoming_slots;
                self.unplaced_owners = else_result.3;
                Ok(Some(value))
            }
            (Some(then_value), Some(else_value)) => {
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
                let merge =
                    self.new_block(origin(expression_origin.raw(), self.next_position), false)?;
                let result_owned = is_owned_value(&result_type);
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
                let then_arguments = edge_arguments(then_value, &bindings, &then_result.2)?;
                let else_arguments = edge_arguments(else_value, &bindings, &else_result.2)?;
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
                self.unplaced_owners = if result_owned {
                    vec![result]
                } else {
                    Vec::new()
                };
                Ok(Some(result))
            }
        }
    }
}
