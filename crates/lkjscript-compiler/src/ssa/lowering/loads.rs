use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_load(
        &mut self,
        binding: hir::BindingRef,
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        match binding.storage {
            BindingStorage::Local(_) => self
                .env
                .get(&binding.binding)
                .copied()
                .map(Some)
                .ok_or_else(|| {
                    Error::msg(format!(
                        "HIR binding {} is not in SSA environment",
                        binding.binding.raw()
                    ))
                }),
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
        })
    }

    pub(in crate::ssa) fn forget_consumed_ref_mut_arguments(&mut self, arguments: &[Expr]) {
        for argument in arguments {
            if !matches!(argument.ty, hir::Type::RefMut(_)) {
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
            let Some(value) = self.lower_expr(argument)? else {
                return Ok(None);
            };
            values.push(value);
        }
        Ok(Some(values))
    }
}
