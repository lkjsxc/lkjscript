use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_load(
        &mut self,
        binding: hir::BindingRef,
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        match binding.storage {
            BindingStorage::Local(_) => {
                let source = self.env.get(&binding.binding).copied().ok_or_else(|| {
                    Error::msg(format!(
                        "HIR binding {} is not in SSA environment",
                        binding.binding.raw()
                    ))
                })?;
                let ty = lower_type(&expression.ty, self.product_ids)?;
                let Some(placement) = self.current_placement else {
                    return Ok(Some(source));
                };
                if !self.structural.is_owned(&ty)
                    || placement.storage == StructuralStorage::BorrowedView
                {
                    return Ok(Some(source));
                }
                let representation = self
                    .structural
                    .representation_by_route(
                        &ty,
                        placement.route,
                        StructuralValueCategory::Owner,
                        placement.storage,
                    )
                    .ok_or_else(|| {
                        Error::msg("structural load placement has no exact owner representation")
                    })?;
                self.append(
                    ty,
                    InstructionKind::StructuralCopy {
                        representation,
                        value: source,
                    },
                    EffectSet::ALLOCATES,
                    expression.origin,
                )
                .map(Some)
            }
            BindingStorage::Function => {
                let target = self
                    .function_ids
                    .get(&binding.binding)
                    .copied()
                    .ok_or_else(|| {
                        Error::msg(format!(
                            "HIR function binding {} has no SSA FunctionId",
                            binding.binding.raw()
                        ))
                    })?;
                let ty = lower_type(&expression.ty, self.product_ids)?;
                self.append(
                    ty,
                    InstructionKind::FunctionRef(target),
                    EffectSet::PURE,
                    expression.origin,
                )
                .map(Some)
            }
        }
    }

    pub(in crate::ssa) fn lower_instantiation(
        &self,
        instantiation: &hir::GenericInstantiation,
        witness_parameters: &[MemoryWitnessParameter],
    ) -> Result<GenericInstantiation> {
        Ok(GenericInstantiation {
            substitutions: instantiation
                .substitutions
                .iter()
                .map(|substitution| {
                    Ok(TypeSubstitution {
                        parameter: substitution.parameter.clone(),
                        ty: lower_type(&substitution.ty, self.product_ids)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            witnesses: instantiation
                .witnesses
                .iter()
                .map(|witness| {
                    Ok(TraitWitness {
                        trait_id: TraitId::new(witness.trait_id.raw()),
                        ty: lower_type(&witness.ty, self.product_ids)?,
                        kind: match witness.kind {
                            hir::TraitWitnessKind::AutoTrait => TraitWitnessKind::AutoTrait,
                            hir::TraitWitnessKind::Explicit(id) => {
                                TraitWitnessKind::Explicit(ImplId::new(id.raw()))
                            }
                        },
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            memory_witnesses: witness_parameters
                .iter()
                .map(|requirement| {
                    let substitution = instantiation
                        .substitutions
                        .iter()
                        .find(|substitution| substitution.parameter == requirement.parameter)
                        .ok_or_else(|| Error::msg("generic call lost witness substitution"))?;
                    let ty = lower_type(&substitution.ty, self.product_ids)?;
                    let descriptor = self
                        .structural
                        .witnesses
                        .iter()
                        .find(|descriptor| descriptor.ty == ty)
                        .ok_or_else(|| {
                            Error::msg(format!(
                                "generic substitution {} has no executable memory witness",
                                substitution.parameter
                            ))
                        })?;
                    Ok(MemoryWitnessBinding {
                        parameter: requirement.parameter.clone(),
                        witness: descriptor.id,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        })
    }

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
        let mut values = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let incoming_unplaced = self.unplaced_owners.clone();
            let Some(value) = self.lower_expr(argument)? else {
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
}
