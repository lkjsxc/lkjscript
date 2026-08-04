impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn forget_consumed_ref_mut_arguments(&mut self, arguments: &[Expr]) {
        for argument in arguments {
            if !matches!(argument.ty, hir::Type::ByteSliceMut) {
                continue;
            }
            if let ExprKind::Load(binding) = &argument.kind {
                self.env.remove(&binding.binding);
            }
        }
    }

    pub(in crate::ssa) fn lower_arguments(
        &mut self,
        arguments: &[Expr],
    ) -> Result<Option<Vec<ValueId>>> {
        self.lower_arguments_with_modes(arguments, None)
    }

    pub(in crate::ssa) fn lower_call_arguments(
        &mut self,
        arguments: &[Expr],
        modes: Option<&[MemoryParameterMode]>,
    ) -> Result<Option<Vec<ValueId>>> {
        if modes.is_some_and(|modes| modes.len() != arguments.len()) {
            return Err(Error::msg(
                "verified call parameter modes do not match argument arity",
            ));
        }
        self.lower_arguments_with_modes(arguments, modes)
    }

    fn lower_arguments_with_modes(
        &mut self,
        arguments: &[Expr],
        modes: Option<&[MemoryParameterMode]>,
    ) -> Result<Option<Vec<ValueId>>> {
        let mut values = Vec::with_capacity(arguments.len());
        for (index, argument) in arguments.iter().enumerate() {
            let incoming_unplaced = self.unplaced_owners.clone();
            let mode = modes.and_then(|modes| modes.get(index)).copied();
            let Some(value) = self.lower_argument(argument, mode)? else {
                return Ok(None);
            };
            let successor_unplaced = self.unplaced_owners.clone();
            for (previous_index, previous) in values.iter_mut().enumerate() {
                let expression = &arguments[previous_index];
                let loaded_successor = match &expression.kind {
                    ExprKind::Load(reference) => self.env.get(&reference.binding),
                    _ => None,
                };
                let needs_relink = incoming_unplaced.contains(previous)
                    || loaded_successor.is_some_and(|successor| successor != previous);
                if needs_relink && is_owned_value(self.structural, &self.value_type(*previous)?) {
                    *previous = self.structural_successor_value(
                        expression,
                        *previous,
                        &incoming_unplaced,
                        &successor_unplaced,
                    )?;
                }
            }
            values.push(value);
        }
        Ok(Some(values))
    }

    fn lower_argument(
        &mut self,
        argument: &Expr,
        mode: Option<MemoryParameterMode>,
    ) -> Result<Option<ValueId>> {
        let borrowed_local = mode == Some(MemoryParameterMode::BorrowShared)
            && matches!(
                &argument.kind,
                ExprKind::Load(hir::BindingRef {
                    storage: BindingStorage::Local(_),
                    ..
                })
            );
        let previous = self.borrowed_call_argument;
        self.borrowed_call_argument = borrowed_local;
        let result = self.lower_expr(argument);
        self.borrowed_call_argument = previous;
        result
    }
}
