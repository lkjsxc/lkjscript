impl<'a> Producer<'a> {
    fn add_obligation(
        &mut self,
        entry: MemoryEntryId,
        kind: MemoryObligationKind,
        drop_glue: Option<MemoryDropGlueId>,
        drop_path: Option<MemoryDropPathId>,
    ) -> Result<()> {
        self.charge_obligations(1)?;
        let id = MemoryObligationId::new(
            u64::try_from(self.obligations.len())
                .map_err(|_| Error::msg("HIR memory-plan obligation identity exceeds u64"))?,
        );
        self.obligations.push(MemoryObligation {
            id,
            function: self.current_function,
            entry,
            kind,
            drop_glue,
            drop_path,
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
        self.entries
            .try_reserve(1)
            .map_err(|_| Error::host("HIR memory-plan entry allocation failed"))?;
        let id = MemoryEntryId::new(
            u64::try_from(self.entries.len())
                .map_err(|_| Error::msg("HIR memory-plan entry identity exceeds u64"))?,
        );
        let type_fact = self.type_planner.intern(ty)?;
        let fact = self.type_planner.fact(type_fact)?.clone();
        let (mode, execution, execution_cutover) = memory_mode(ty, &fact, effects, escape)?;
        let owns_glue = fact.mode != MemoryAggregateMode::Copy;
        let drop_path = owns_glue.then_some(fact.drop_path).flatten();
        let expression_index = match &subject {
            MemorySubject::Expression {
                expression,
                parent,
                child_index,
                ..
            } => Some((*expression, *parent, *child_index)),
            _ => None,
        };
        let place_index = match &subject {
            MemorySubject::Place {
                function,
                binding,
                place,
            } => Some(((*function, *binding), *place)),
            _ => None,
        };
        self.entries.push(MemoryPlanEntry {
            id,
            subject,
            ty: memory_type(ty),
            effects,
            mode,
            type_fact,
            root_projection: fact.root_projection,
            destination: None,
            copy_share: fact.copy_share,
            borrow_scope: None,
            drop_path,
            execution,
            execution_cutover,
            origin,
            drop_glue: owns_glue.then_some(fact.drop_glue).flatten(),
        });
        if let Some((expression, parent, child_index)) = expression_index {
            if self.expression_entries.insert(expression, id).is_some() {
                return Err(Error::msg("HIR memory-plan expression entry index is duplicated"));
            }
            if let Some(parent) = parent {
                let children = self.child_entries.entry(parent).or_default();
                children
                    .try_reserve(1)
                    .map_err(|_| Error::host("HIR memory-plan child index allocation failed"))?;
                children.push((child_index, expression, drop_path));
            }
        }
        if let Some((key, place)) = place_index {
            if self.places_by_binding.insert(key, place).is_some() {
                return Err(Error::msg("HIR memory-plan place binding index is duplicated"));
            }
        }
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
        let mut loads_by_binding: BTreeMap<(MemoryFunctionId, u64), Vec<MemoryExpressionId>> =
            BTreeMap::new();
        for usage in &self.uses {
            if usage.kind == MemoryUseKind::Load {
                let expressions = loads_by_binding
                    .entry((usage.function, usage.binding))
                    .or_default();
                expressions
                    .try_reserve(1)
                    .map_err(|_| Error::host("HIR memory-plan loan-use index allocation failed"))?;
                expressions.push(usage.expression);
            }
        }
        for expressions in loads_by_binding.values_mut() {
            expressions.sort_unstable();
        }
        for loan in &mut self.loans {
            let (semantic_uses, end_after) = if let Some(binding) = loan.binding {
                let expressions = loads_by_binding
                    .get(&(loan.function, binding))
                    .ok_or_else(|| Error::msg("HIR memory-plan loan has no semantic reference use"))?;
                let first = expressions.partition_point(|expression| *expression <= loan.expression);
                let matching = &expressions[first..];
                let last_use = *matching
                    .last()
                    .ok_or_else(|| Error::msg("HIR memory-plan loan has no semantic reference use"))?;
                let end_after = self
                    .expression_parents
                    .get(&last_use)
                    .copied()
                    .flatten()
                    .ok_or_else(|| {
                        Error::msg("HIR memory-plan reference use has no complete call expression")
                    })?;
                (
                    u64::try_from(matching.len())
                        .map_err(|_| Error::msg("HIR memory-plan loan use count exceeds u64"))?,
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
        observe(&mut self.work.functions, amount, "functions")
    }
    fn charge_entries(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.entries, amount, "entries")
    }
    fn charge_expressions(&mut self, amount: usize) -> Result<()> {
        let amount = u64::try_from(amount)
            .map_err(|_| Error::msg("HIR memory-plan expression charge exceeds u64"))?;
        self.work.expressions = self
            .work
            .expressions
            .checked_add(amount)
            .ok_or_else(|| Error::msg("HIR memory-plan expression work overflow"))?;
        Ok(())
    }
    fn charge_uses(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.uses, amount, "uses")
    }
    fn charge_loans(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.loans, amount, "loans")
    }
    fn charge_constants(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.constants, amount, "constants")
    }
    fn charge_calls(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.calls, amount, "calls")
    }
    fn charge_obligations(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.obligations, amount, "obligations")
    }
}
