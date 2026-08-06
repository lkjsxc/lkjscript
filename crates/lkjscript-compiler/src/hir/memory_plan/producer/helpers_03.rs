impl Producer<'_> {
    fn planned_parameter_mode(
        &mut self,
        ty: &Type,
        consumed: bool,
    ) -> Result<MemoryParameterMode> {
        if matches!(ty, Type::ByteSlice) { return Ok(MemoryParameterMode::BorrowShared); }
        if matches!(ty, Type::ByteSliceMut) { return Ok(MemoryParameterMode::BorrowExclusive); }
        if matches!(ty, Type::Resource(_)) {
            return Ok(if consumed { MemoryParameterMode::Consume }
                else { MemoryParameterMode::BorrowExclusive });
        }
        let id = self.type_planner.intern(ty)?;
        let fact = self.type_planner.fact(id)?;
        Ok(if fact.closure.class != MemoryClosureClass::Deterministic
            || matches!(ty, Type::List(_))
        {
            MemoryParameterMode::Copy
        } else {
            match fact.mode {
                MemoryAggregateMode::Copy => MemoryParameterMode::Copy,
                MemoryAggregateMode::ImmutableValue => MemoryParameterMode::BorrowShared,
                MemoryAggregateMode::Affine => MemoryParameterMode::Consume,
            }
        })
    }

    fn planned_result_mode(&mut self, ty: &Type) -> Result<MemoryResultMode> {
        let id = self.type_planner.intern(ty)?;
        let fact = self.type_planner.fact(id)?;
        if fact.contains_borrow {
            return Err(Error::msg(format!(
                "LKJ-MEM-BORROWED-RESULT type={:?} reason=borrowed result/escape",
                memory_type(ty),
            )));
        }
        if matches!(ty, Type::Resource(_)) { return Ok(MemoryResultMode::External); }
        Ok(if fact.closure.class != MemoryClosureClass::Deterministic
            || fact.mode == MemoryAggregateMode::Copy
            || matches!(ty, Type::List(_))
        {
            MemoryResultMode::Trivial
        } else { MemoryResultMode::Owned })
    }

    fn finish_type_work(&mut self) -> Result<()> {
        self.work.type_nodes = u64::try_from(self.type_planner.facts.len())
            .map_err(|_| Error::msg("HIR memory-plan type facts exceed u64"))?;
        self.work.witnesses = u64::try_from(self.type_planner.witnesses.len())
            .map_err(|_| Error::msg("HIR memory-plan witnesses exceed u64"))?;
        if self.work.witnesses != self.work.type_nodes {
            return Err(Error::msg("HIR memory-plan witness table is not exact"));
        }
        self.work.type_edges = self.type_planner.graph.edges;
        self.work.scc_work = self.type_planner.graph.scc_work;
        self.work.aggregate_fields = self.type_planner.fields;
        self.work.aggregate_variants = self.type_planner.variants;
        self.work.destinations = u64::try_from(self.destinations.len())
            .map_err(|_| Error::msg("HIR memory-plan destinations exceed u64"))?;
        self.work.borrow_scopes = u64::try_from(self.borrow_scopes.len())
            .map_err(|_| Error::msg("HIR memory-plan borrow scopes exceed u64"))?;
        self.work.drop_paths = u64::try_from(self.type_planner.drop_paths.len())
            .map_err(|_| Error::msg("HIR memory-plan drop paths exceed u64"))?;
        Ok(())
    }

    fn reject_partial_projection(&mut self, expression: &Expr) -> Result<()> {
        let source = match &expression.kind {
            ExprKind::ProductField { value, .. }
            | ExprKind::WithProductField { value, .. }
            | ExprKind::EnumField { value, .. }
            | ExprKind::EnumUnwrap { value, .. } => value,
            _ => return Ok(()),
        };
        if !matches!(source.kind, ExprKind::Load(_)) { return Ok(()); }
        let id = self.type_planner.intern(&expression.ty)?;
        if self.type_planner.fact(id)?.mode == MemoryAggregateMode::Affine {
            return Err(Error::msg(format!(
                "LKJ-MEM-PARTIAL-MOVE type={:?} path={:?} reason=affine aggregate field projection",
                memory_type(&expression.ty), expression_kind(&expression.kind),
            )));
        }
        Ok(())
    }

    fn add_destination(&mut self, expression: &Expr, expression_id: MemoryExpressionId) -> Result<()> {
        let entry_id = *self
            .expression_entries
            .get(&expression_id)
            .ok_or_else(|| Error::msg("aggregate destination lost expression entry"))?;
        let entry_index = entry_id.index().ok_or_else(|| Error::msg("aggregate entry exceeds usize"))?;
        let type_fact = self.entries[entry_index].type_fact;
        let fact = self.type_planner.fact(type_fact)?.clone();
        let mut children = self
            .child_entries
            .remove(&expression_id)
            .unwrap_or_default();
        children.sort_by_key(|item| item.0);
        let (field_count, active_payload) = destination_shape(
            self.program,
            &self.products_by_id,
            &self.enums_by_id,
            expression,
        )?;
        let field_count_index = usize::try_from(field_count)
            .map_err(|_| Error::msg("destination field count exceeds host usize"))?;
        if children.len() != field_count_index {
            return Err(Error::msg("LKJ-MEM-INCOMPLETE-DESTINATION field count mismatch"));
        }
        let id = MemoryDestinationId::new(u64::try_from(self.destinations.len())
            .map_err(|_| Error::msg("HIR memory-plan destination identity exceeds u64"))?);
        let initialized_order: Vec<u64> = (0..field_count).collect();
        let fields = children.into_iter().map(|(index, expression, drop_path)| {
            MemoryDestinationField { index, expression, drop_path }
        }).collect();
        let (kind, execution, execution_cutover) = match fact.closure.class {
            MemoryClosureClass::Deterministic => (
                MemoryDestinationKind::CutoverRequired,
                MemoryExecution::CutoverRequired,
                execution_cutover(&expression.ty),
            ),
            MemoryClosureClass::RegionClosed => (
                MemoryDestinationKind::OrdinaryRegion,
                MemoryExecution::Current,
                None,
            ),
            MemoryClosureClass::Unresolved | MemoryClosureClass::IllegalDomainBridge => (
                MemoryDestinationKind::UnsupportedRuntime,
                MemoryExecution::CutoverRequired,
                None,
            ),
        };
        self.destinations.push(MemoryDestinationPlan { id, function: self.current_function,
            expression: expression_id, kind, execution, execution_cutover, type_fact,
            field_count, fields,
            active_payload, initialized_order: initialized_order.clone(),
            reverse_abort_cleanup: initialized_order.into_iter().rev().collect() });
        self.entries[entry_index].destination = Some(id);
        Ok(())
    }
}
