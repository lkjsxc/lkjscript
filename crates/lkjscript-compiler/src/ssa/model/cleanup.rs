impl CleanupPlan {
    pub(in crate::ssa) fn new(
        plan: &HirMemoryPlan,
        function: MemoryFunctionId,
        products: &HashMap<String, ProductId>,
        structural: &StructuralMemoryMetadata,
    ) -> Result<Self> {
        let function_plan = plan
            .function(function)
            .ok_or_else(|| Error::msg("HIR memory plan lost an SSA function"))?;
        let mut loan_ends: BTreeMap<u64, Vec<SsaLoanId>> = BTreeMap::new();
        for loan in plan.loans.iter().filter(|loan| loan.function == function) {
            loan_ends
                .entry(loan.end_after.raw())
                .or_default()
                .push(SsaLoanId::new(loan.loan));
        }
        let mut place_glues = BTreeMap::new();
        let mut place_drop_classes = BTreeMap::new();
        for obligation in plan
            .obligations
            .iter()
            .filter(|obligation| obligation.function == function)
        {
            if matches!(obligation.kind, MemoryObligationKind::EndBorrow) {
                continue;
            }
            let entry = plan
                .entry(obligation.entry)
                .ok_or_else(|| Error::msg("HIR memory obligation lost its entry"))?;
            let MemorySubject::Place { place, .. } = entry.subject else {
                return Err(Error::msg("HIR drop obligation does not name a place"));
            };
            if entry.execution == crate::memory_plan::MemoryExecution::CutoverRequired {
                if entry.execution_cutover.is_none() {
                    return Err(Error::msg("HIR execution cutover lost its exact authority"));
                }
                continue;
            }
            let glue = obligation
                .drop_glue
                .and_then(|id| plan.drop_glues.iter().find(|glue| glue.id == id))
                .ok_or_else(|| Error::msg("HIR drop obligation lost closed glue identity"))?;
            let glue = match &glue.kind {
                MemoryDropGlueKind::ByteVector => DropGlueIdentity::ByteVector,
                MemoryDropGlueKind::Bytes => DropGlueIdentity::Bytes,
                MemoryDropGlueKind::Resource(kind) => DropGlueIdentity::Resource(*kind),
                MemoryDropGlueKind::String
                | MemoryDropGlueKind::Path
                | MemoryDropGlueKind::Product(_)
                | MemoryDropGlueKind::Enum { .. } => {
                    let ty = glue_type(&glue.kind, products)?.ok_or_else(|| {
                        Error::msg("structural HIR glue lost its exact semantic type")
                    })?;
                    structural_glue(structural, &ty)?
                }
            };
            let place = SsaPlaceId::new(place);
            let drop_class = obligation
                .drop_class
                .ok_or_else(|| Error::msg("HIR place obligation lost its drop class"))?;
            if drop_class == MemoryDropClass::Open {
                return Err(Error::msg("open HIR drop class reached SSA lowering"));
            }
            if place_glues.insert(place, glue).is_some()
                || place_drop_classes.insert(place, drop_class).is_some()
            {
                return Err(Error::msg(
                    "HIR memory plan duplicates a place drop obligation",
                ));
            }
        }
        let mut places = plan
            .entries
            .iter()
            .filter_map(|entry| match entry.subject {
                MemorySubject::Place {
                    function: owner,
                    place,
                    binding,
                } if owner == function => Some((entry, place, binding)),
                _ => None,
            })
            .map(|(entry, place, binding)| {
                let id = SsaPlaceId::new(place);
                Ok(PlaceMetadata {
                    id,
                    binding: SsaBindingId::new(binding),
                    ty: lower_memory_type(&entry.ty, products)?,
                    drop_glue: place_glues.get(&id).copied(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        places.sort_by_key(|place| place.id);
        if places
            .iter()
            .enumerate()
            .any(|(index, place)| place.id.index() != Some(index))
        {
            return Err(Error::msg(
                "HIR memory plan has non-dense SSA place metadata",
            ));
        }
        let placement_routes = plan.value_placements.iter()
            .map(|placement| {
                Ok((placement.expression.raw(), ActiveValuePlacement {
                    route: placement.representation.as_bytes(),
                    storage: placement_storage(placement.storage)?,
                }))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let mut call_parameter_modes = BTreeMap::new();
        for call in plan.calls.iter().filter(|call| call.function == function) {
            if call_parameter_modes
                .insert(call.expression.raw(), call.parameters.clone())
                .is_some()
            {
                return Err(Error::msg("HIR memory plan duplicates a call expression"));
            }
        }
        Ok(Self {
            next_expression: function_plan.body.raw(),
            placement_routes,
            call_parameter_modes,
            loan_ends,
            places,
            place_drop_classes,
        })
    }
    pub(in crate::ssa) fn begin_expression(&mut self) -> Result<MemoryExpressionId> {
        let id = MemoryExpressionId::new(self.next_expression);
        self.next_expression = self
            .next_expression
            .checked_add(1)
            .ok_or_else(|| Error::msg("SSA HIR memory expression identity overflow"))?;
        Ok(id)
    }

    pub(in crate::ssa) fn placement(
        &self,
        expression: MemoryExpressionId,
    ) -> Option<ActiveValuePlacement> {
        self.placement_routes.get(&expression.raw()).copied()
    }
}

fn placement_storage(value: crate::memory_plan::MemoryDomain) -> Result<StructuralStorage> {
    Ok(match value {
        crate::memory_plan::MemoryDomain::Inline => StructuralStorage::Inline,
        crate::memory_plan::MemoryDomain::Static => StructuralStorage::Static,
        crate::memory_plan::MemoryDomain::Stack => StructuralStorage::Stack,
        crate::memory_plan::MemoryDomain::CallerDestination => StructuralStorage::CallerDestination,
        crate::memory_plan::MemoryDomain::UniqueStructural => StructuralStorage::UniqueStructural,
        crate::memory_plan::MemoryDomain::OrdinaryRegion => StructuralStorage::OrdinaryRegion,
        crate::memory_plan::MemoryDomain::SealedRegion => StructuralStorage::SealedRegion,
        crate::memory_plan::MemoryDomain::BorrowedView => StructuralStorage::BorrowedView,
        crate::memory_plan::MemoryDomain::ExternalResource => StructuralStorage::ExternalResource,
        crate::memory_plan::MemoryDomain::UnsupportedRuntime => {
            return Err(Error::msg("unsupported value placement reached SSA cleanup"));
        }
    })
}
