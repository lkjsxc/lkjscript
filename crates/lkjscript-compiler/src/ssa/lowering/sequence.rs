use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_sequence(
        &mut self,
        expressions: &[Expr],
        sequence_origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let mut result = None;
        for (index, expression) in expressions.iter().enumerate() {
            result = self.lower_expr(expression)?;
            let Some(value) = result else {
                return Ok(None);
            };
            if index.saturating_add(1) < expressions.len() {
                if let hir::Type::Param(parameter) = &expression.ty {
                    if self
                        .signature
                        .memory_witness_parameters
                        .iter()
                        .any(|requirement| {
                            requirement.parameter == *parameter
                                && requirement
                                    .operations
                                    .contains(&lkjscript_contracts::MemoryWitnessOperation::Dispose)
                        })
                    {
                        let _disposed = self.append(
                            SsaType::Unit,
                            InstructionKind::MemoryWitnessDispose {
                                parameter: parameter.clone(),
                                value,
                            },
                            EffectSet::PURE,
                            expression.origin,
                        )?;
                    }
                }
            }
            if index.saturating_add(1) < expressions.len() && expression.ty == hir::Type::ByteVector
            {
                let place = discarded_move_place(expression).ok_or_else(|| {
                    Error::msg(
                        "discarded byte-vector temporary requires an explicit whole-place Move",
                    )
                })?;
                let glue = self
                    .places
                    .get(place.index().unwrap_or(usize::MAX))
                    .and_then(|place| place.drop_glue)
                    .ok_or_else(|| Error::msg("discarded byte-vector Move lost drop glue"))?;
                let _drop = self.append(
                    SsaType::Unit,
                    InstructionKind::Drop {
                        place,
                        value,
                        glue,
                        kind: DropEventKind::ImplicitCleanup,
                    },
                    EffectSet::PURE,
                    expression.origin,
                )?;
                result = Some(value);
            }
        }
        if let Some(result) = result {
            Ok(Some(result))
        } else {
            self.constant(SsaType::Unit, Constant::Unit, sequence_origin)
                .map(Some)
        }
    }

    pub(in crate::ssa) fn lower_let(
        &mut self,
        bindings: &[LocalDefinition],
        body: &Expr,
    ) -> Result<Option<ValueId>> {
        let mut previous = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let Some(value) = self.lower_expr(&binding.value)? else {
                for (binding, previous_value, previous_slot) in previous.into_iter().rev() {
                    restore(&mut self.env, binding, previous_value);
                    restore(&mut self.slots, binding, previous_slot);
                }
                return Ok(None);
            };
            let place_ty = lower_type(&binding.value.ty, self.product_ids)?;
            self.register_place(binding.place, binding.binding, place_ty)?;
            if !binding.static_bytes {
                self.initialize_owned_place(binding.binding, value, binding.value.origin)?;
            }
            previous.push((
                binding.binding,
                self.env.insert(binding.binding, value),
                self.slots.insert(
                    binding.binding,
                    u64::try_from(binding.slot)
                        .map_err(|_| Error::msg("HIR local slot exceeds u64"))?,
                ),
            ));
        }
        let result = self.lower_expr(body)?;
        if result.is_some() {
            for (binding, _, _) in previous.iter().rev() {
                self.end_owned_place(*binding, body.origin)?;
            }
        }
        for (binding, previous_value, previous_slot) in previous.into_iter().rev() {
            restore(&mut self.env, binding, previous_value);
            restore(&mut self.slots, binding, previous_slot);
        }
        Ok(result)
    }
}

fn discarded_move_place(expression: &Expr) -> Option<SsaPlaceId> {
    match &expression.kind {
        ExprKind::Move { place, .. } => Some(SsaPlaceId::new(place.raw())),
        ExprKind::Do(expressions) => expressions.last().and_then(discarded_move_place),
        ExprKind::If {
            then_branch,
            else_branch,
            ..
        } => {
            let then_place = discarded_move_place(then_branch)?;
            (discarded_move_place(else_branch)? == then_place).then_some(then_place)
        }
        _ => None,
    }
}
