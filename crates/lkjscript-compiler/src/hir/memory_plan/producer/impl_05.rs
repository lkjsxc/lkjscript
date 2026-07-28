impl<'a> Producer<'a> {
    fn add_obligation(
        &mut self,
        entry: MemoryEntryId,
        kind: MemoryObligationKind,
        drop_glue: Option<MemoryDropGlueId>,
    ) -> Result<()> {
        self.charge_obligations(1)?;
        let id = MemoryObligationId::new(
            u32::try_from(self.obligations.len())
                .map_err(|_| Error::msg("HIR memory-plan obligation identity exceeds u32"))?,
        );
        self.obligations.push(MemoryObligation {
            id,
            function: self.current_function,
            entry,
            kind,
            drop_glue,
            drop_class: (!matches!(kind, MemoryObligationKind::EndBorrow))
                .then_some(MemoryDropClass::Static),
        });
        Ok(())
    }
    fn add_entry(
        &mut self,
        subject: MemorySubject,
        ty: &Type,
        effects: u16,
        escape: MemoryEscape,
        origin: MemoryOrigin,
    ) -> Result<MemoryEntryId> {
        self.charge_entries(1)?;
        let id = MemoryEntryId::new(
            u32::try_from(self.entries.len())
                .map_err(|_| Error::msg("HIR memory-plan entry identity exceeds u32"))?,
        );
        let (mode, legacy_family, drop_glue) = memory_mode(ty, effects, escape);
        self.entries.push(MemoryPlanEntry {
            id,
            subject,
            ty: memory_type(ty),
            effects,
            mode,
            origin,
            legacy_family: legacy_family.map(str::to_owned),
            drop_glue,
        });
        Ok(id)
    }
    fn next_expression(&mut self) -> Result<MemoryExpressionId> {
        self.charge_expressions(1)?;
        let id = MemoryExpressionId::new(self.next_expression);
        self.next_expression = self
            .next_expression
            .checked_add(1)
            .ok_or_else(|| Error::msg("HIR memory-plan expression identity overflow"))?;
        Ok(id)
    }
    fn finish_loans(&mut self) -> Result<()> {
        for loan in &mut self.loans {
            let (semantic_uses, end_after) = if let Some(binding) = loan.binding {
                let matching: Vec<_> = self
                    .uses
                    .iter()
                    .filter(|item| {
                        item.function == loan.function
                            && item.binding == binding
                            && item.kind == MemoryUseKind::Load
                            && item.expression > loan.expression
                    })
                    .collect();
                let last_use = matching.last().map(|item| item.expression).ok_or_else(|| {
                    Error::msg("HIR memory-plan loan has no semantic reference use")
                })?;
                let end_after = self
                    .expression_parents
                    .get(&last_use)
                    .copied()
                    .flatten()
                    .ok_or_else(|| {
                        Error::msg("HIR memory-plan reference use has no complete call expression")
                    })?;
                (
                    u32::try_from(matching.len())
                        .map_err(|_| Error::msg("HIR memory-plan loan use count exceeds u32"))?,
                    end_after,
                )
            } else {
                let parent = self
                    .expression_parents
                    .get(&loan.expression)
                    .copied()
                    .flatten()
                    .ok_or_else(|| {
                        Error::msg("temporary HIR loan has no enclosing call expression")
                    })?;
                (1, parent)
            };
            loan.semantic_uses = semantic_uses;
            loan.end_after = end_after;
        }
        Ok(())
    }
    fn signature(&self, id: MemoryFunctionId) -> Result<&FunctionMemorySignature> {
        id.index()
            .and_then(|index| self.signatures.get(index))
            .filter(|signature| signature.function == id)
            .ok_or_else(|| Error::msg("HIR memory-plan function signature is missing"))
    }
    fn binding_type(&self, binding: BindingId) -> Result<&Type> {
        self.program
            .binding(binding)
            .map(|binding| &binding.ty)
            .ok_or_else(|| Error::msg("HIR memory-plan references unknown binding"))
    }
    fn charge_functions(&mut self, amount: usize) -> Result<()> {
        charge(
            &mut self.work.functions,
            amount,
            MAX_MEMORY_PLAN_FUNCTIONS,
            "functions",
        )
    }
    fn charge_entries(&mut self, amount: usize) -> Result<()> {
        charge(
            &mut self.work.entries,
            amount,
            MAX_MEMORY_PLAN_ENTRIES,
            "entries",
        )
    }
    fn charge_expressions(&mut self, amount: usize) -> Result<()> {
        charge(
            &mut self.work.expressions,
            amount,
            MAX_MEMORY_PLAN_EXPRESSIONS,
            "expressions",
        )
    }
    fn charge_uses(&mut self, amount: usize) -> Result<()> {
        charge(&mut self.work.uses, amount, MAX_MEMORY_PLAN_USES, "uses")
    }
    fn charge_loans(&mut self, amount: usize) -> Result<()> {
        charge(&mut self.work.loans, amount, MAX_MEMORY_PLAN_LOANS, "loans")
    }
    fn charge_constants(&mut self, amount: usize) -> Result<()> {
        charge(
            &mut self.work.constants,
            amount,
            MAX_MEMORY_PLAN_CONSTANTS,
            "constants",
        )
    }
    fn charge_calls(&mut self, amount: usize) -> Result<()> {
        charge(&mut self.work.calls, amount, MAX_MEMORY_PLAN_CALLS, "calls")
    }
    fn charge_obligations(&mut self, amount: usize) -> Result<()> {
        charge(
            &mut self.work.obligations,
            amount,
            MAX_MEMORY_PLAN_OBLIGATIONS,
            "obligations",
        )
    }
}
