use crate::ssa::*;

mod conditional;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn begin_memory_expression(&mut self) -> Result<MemoryExpressionId> {
        self.cleanup.begin_expression()
    }

    pub(in crate::ssa) fn finish_memory_expression(
        &mut self,
        expression: MemoryExpressionId,
        expression_origin: hir::Origin,
    ) -> Result<()> {
        let loans = self
            .cleanup
            .loan_ends
            .remove(&expression.raw())
            .unwrap_or_default();
        for loan in loans {
            let active = self.active_loans.get(&loan).copied().ok_or_else(|| {
                Error::msg(format!(
                    "HIR memory plan ends unavailable SSA LoanId {}",
                    loan.raw()
                ))
            })?;
            let _event = self.append(
                SsaType::Unit,
                InstructionKind::EndBorrow {
                    place: active.place,
                    loan,
                    value: active.value,
                },
                EffectSet::PURE,
                expression_origin,
            )?;
            self.active_loans.remove(&loan);
        }
        Ok(())
    }

    pub(in crate::ssa) fn record_active_loan(
        &mut self,
        loan: SsaLoanId,
        place: SsaPlaceId,
        kind: SsaBorrowKind,
        value: ValueId,
    ) -> Result<()> {
        if self
            .active_loans
            .insert(loan, ActiveLoan { place, kind, value })
            .is_some()
        {
            return Err(Error::msg("SSA lowering duplicated an active HIR loan"));
        }
        Ok(())
    }

    pub(in crate::ssa) fn record_explicit_close(
        &mut self,
        binding: BindingId,
        value: ValueId,
        expression_origin: hir::Origin,
    ) -> Result<()> {
        let place = self
            .owned_place_for_binding(binding)?
            .ok_or_else(|| Error::msg("resource close has no owned SSA place obligation"))?;
        let glue = self
            .places
            .get(place.index().unwrap_or(usize::MAX))
            .and_then(|place| place.drop_glue)
            .ok_or_else(|| Error::msg("resource close lost its HIR drop glue"))?;
        if !matches!(glue, DropGlueIdentity::Resource(_)) {
            return Err(Error::msg("resource close has non-resource drop glue"));
        }
        self.env.remove(&binding);
        let _event = self.append(
            SsaType::Unit,
            InstructionKind::Drop {
                place,
                value,
                glue,
                kind: DropEventKind::ExplicitClose,
            },
            EffectSet::PURE,
            expression_origin,
        )?;
        Ok(())
    }

    pub(in crate::ssa) fn end_owned_place(
        &mut self,
        binding: BindingId,
        expression_origin: hir::Origin,
    ) -> Result<()> {
        if !self.active_place_bindings.contains(&binding) {
            return Ok(());
        }
        let Some(place) = self.owned_place_for_binding(binding)? else {
            return Ok(());
        };
        let glue = self
            .places
            .get(place.index().unwrap_or(usize::MAX))
            .and_then(|place| place.drop_glue)
            .ok_or_else(|| Error::msg("owned SSA place lost its HIR drop glue"))?;
        if let Some(value) = self.env.get(&binding).copied() {
            if glue == DropGlueIdentity::Resource(lkjscript_core::ResourceKind::InputStream) {
                return Err(Error::msg(format!(
                    "borrowed standard-input resource reaches guest-owned place-end for binding {}",
                    binding.raw()
                )));
            }
            let _event = self.append(
                SsaType::Unit,
                InstructionKind::Drop {
                    place,
                    value,
                    glue,
                    kind: DropEventKind::ImplicitCleanup,
                },
                EffectSet::PURE,
                expression_origin,
            )?;
            self.env.remove(&binding);
        }
        let _end = self.append(
            SsaType::Unit,
            InstructionKind::PlaceEnd { place },
            EffectSet::PURE,
            expression_origin,
        )?;
        self.active_place_bindings
            .retain(|active| *active != binding);
        Ok(())
    }

    pub(in crate::ssa) fn cleanup_places_to(
        &mut self,
        depth: usize,
        expression_origin: hir::Origin,
    ) -> Result<()> {
        let bindings: Vec<_> = self
            .active_place_bindings
            .iter()
            .skip(depth)
            .copied()
            .collect();
        for binding in bindings.into_iter().rev() {
            self.end_owned_place(binding, expression_origin)?;
        }
        Ok(())
    }

    pub(in crate::ssa) fn cleanup_all_places(
        &mut self,
        expression_origin: hir::Origin,
    ) -> Result<()> {
        self.cleanup_places_to(0, expression_origin)
    }
}
