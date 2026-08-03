impl TypePlanner<'_> {
    fn add_witness(
        &mut self,
        ty: &Type,
        derived: &DerivedType,
        root_projection: MemoryRootProjection,
        copy_share: MemoryCopySharePlan,
        drop_glue: Option<MemoryDropGlueId>,
        drop_path: Option<MemoryDropPathId>,
    ) -> Result<MemoryWitnessId> {
        if u64::try_from(self.witnesses.len()).unwrap_or(u64::MAX) >= MAX_MEMORY_PLAN_WITNESSES {
            return Err(Error::msg("HIR memory-plan witnesses exceed bounded maximum"));
        }
        let semantic = self.producer_semantic_descriptor(ty)?;
        let dependencies = self.producer_witness_dependencies(ty)?;
        lkjscript_contracts::validate_executable_dependencies(&semantic, &dependencies)
            .map_err(|error| Error::msg(error.to_string()))?;
        let semantic_contract = lkjscript_contracts::semantic_contract_hash(&semantic)
            .map_err(|error| Error::msg(error.to_string()))?;
        let facts = MemoryWitnessFacts {
            ty: memory_type(ty),
            semantic_contract,
            semantic,
            dependencies,
            requirement: witness_requirement(ty),
            mode: derived.mode,
            capabilities: witness_capabilities(ty, derived),
            closure: derived.closure.clone(),
            root_projection,
            domain: witness_domain(ty, derived),
            copy_share,
            drop_glue: drop_glue.map(|id| self.witness_glue(id)).transpose()?,
            drop_path: drop_path.map(|id| self.witness_path(id)).transpose()?,
            equality: witness_equality(ty),
            process_codec: witness_process_codec(ty, derived),
            list_element: witness_list_element(ty, derived),
            list: self.witness_list(ty, derived)?,
            dynamic_size: witness_dynamic_size(ty),
            contains_borrow: derived.contains_borrow,
            contains_dynamic_owner: derived.contains_dynamic_owner,
            portability: witness_portability(ty),
            contention: witness_contention(ty, derived),
        };
        let semantic_identity = lkjscript_contracts::semantic_type_closure_hash(&facts.semantic)
            .map_err(|error| Error::msg(error.to_string()))?;
        let id = MemoryWitnessId::from_bytes(semantic_identity);
        if self.witnesses.iter().any(|item| item.id == id) {
            return Err(Error::msg("HIR memory-plan produced a duplicate semantic witness"));
        }
        self.witnesses.push(MemoryWitness {
            id,
            group: MemoryWitnessGroupId::from_bytes([0; 32]),
            ordinal: u16::MAX,
            facts,
        });
        Ok(id)
    }

    fn witness_list(
        &self,
        ty: &Type,
        list_derived: &DerivedType,
    ) -> Result<Option<MemoryListWitness>> {
        let Type::List(element) = ty else {
            return Ok(None);
        };
        let id = self
            .memo
            .get(element.as_ref())
            .copied()
            .ok_or_else(|| Error::msg("memory list witness lost its element fact"))?;
        let fact = self.fact(id)?;
        let element_derived = DerivedType {
            mode: fact.mode,
            closure: fact.closure.clone(),
            contains_borrow: fact.contains_borrow,
            contains_dynamic_owner: fact.contains_dynamic_owner,
        };
        Ok(Some(MemoryListWitness {
            element: fact.witness,
            selected: list_derived.closure.class == MemoryClosureClass::RegionClosed,
            eligibility: witness_list_element(element, &element_derived),
            storage: MemoryListStorageKind::SegmentedSessionRegion,
            segment_capacity: 32,
        }))
    }


    fn witness_glue(&self, id: MemoryDropGlueId) -> Result<MemoryDropGlueKind> {
        self.glues
            .get(id.index().unwrap_or(usize::MAX))
            .filter(|item| item.id == id)
            .map(|item| item.kind.clone())
            .ok_or_else(|| Error::msg("memory witness drop glue is missing"))
    }

    fn witness_path(&self, id: MemoryDropPathId) -> Result<Vec<MemoryWitnessDropBranch>> {
        let path = self.drop_paths.get(id.index().unwrap_or(usize::MAX))
            .filter(|item| item.id == id)
            .ok_or_else(|| Error::msg("memory witness drop path is missing"))?;
        path.branches.iter().map(|branch| Ok(MemoryWitnessDropBranch {
            active_variant: branch.active_variant,
            actions: branch.actions.iter().map(|action| Ok(MemoryWitnessDropAction {
                path: action.path.clone(),
                glue: self.witness_glue(action.glue)?,
            })).collect::<Result<Vec<_>>>()?,
        })).collect()
    }
}

include!("witness_policy.rs");
include!("witness_capabilities.rs");
include!("semantic_witness/mod.rs");
