use super::*;

impl VerifiedTypes<'_> {
    pub(crate) fn verify_witness(
        &self,
        ty: &Type,
        derived: &VerifiedDerived,
        root_projection: MemoryRootProjection,
        copy_share: MemoryCopySharePlan,
        drop_glue: Option<MemoryDropGlueId>,
        drop_path: Option<MemoryDropPathId>,
    ) -> Result<MemoryWitnessId> {
        let index = MemoryTypeFactId::new(index_u32(self.expected.len())?);
        let actual = self
            .plan
            .witnesses
            .get(index.index().unwrap_or(usize::MAX))
            .ok_or_else(|| Error::msg("HIR memory plan omitted a reconstructed witness"))?;
        let semantic = self.verified_semantic_descriptor(ty)?;
        let expected_dependencies = self.verified_witness_dependencies(ty)?;
        if expected_dependencies.len() != actual.facts.dependencies.len()
            || expected_dependencies
                .iter()
                .zip(&actual.facts.dependencies)
                .any(|(left, right)| {
                    left.role != right.role
                        || matches!(
                            left.target,
                            lkjscript_contracts::ExecutableMemoryWitnessTarget::LocalMember(_)
                        ) != matches!(
                            right.target,
                            lkjscript_contracts::ExecutableMemoryWitnessTarget::LocalMember(_)
                        )
                })
        {
            return Err(Error::msg(
                "independent HIR memory verifier rejected witness edge roles",
            ));
        }
        let dependencies = actual.facts.dependencies.clone();
        lkjscript_contracts::validate_executable_dependencies(&semantic, &dependencies)
            .map_err(|error| Error::msg(error.to_string()))?;
        let semantic_contract = lkjscript_contracts::semantic_contract_hash(&semantic)
            .map_err(|error| Error::msg(error.to_string()))?;
        let facts = MemoryWitnessFacts {
            ty: verified_memory_type(ty),
            semantic_contract,
            semantic,
            dependencies,
            requirement: verified_witness_requirement(ty),
            mode: derived.mode,
            capabilities: verified_witness_capabilities(ty, derived),
            closure: derived.closure.clone(),
            root_projection,
            domain: verified_witness_domain(ty, derived),
            copy_share,
            drop_glue: drop_glue
                .map(|id| self.verified_witness_glue(id))
                .transpose()?,
            drop_path: drop_path
                .map(|id| self.verified_witness_path(id))
                .transpose()?,
            equality: verified_witness_equality(ty),
            process_codec: verified_witness_process_codec(ty, derived),
            list_element: verified_witness_list_element(ty, derived),
            list: self.verified_witness_list(ty, derived)?,
            dynamic_size: verified_witness_dynamic_size(ty),
            contains_borrow: derived.contains_borrow,
            contains_dynamic_owner: derived.contains_dynamic_owner,
            portability: verified_witness_portability(ty),
            contention: verified_witness_contention(ty, derived),
        };
        if actual.facts != facts {
            return Err(Error::msg(
                "independent HIR memory verifier rejected exact memory witness facts",
            ));
        }
        Ok(actual.id)
    }

    fn verified_witness_list(
        &self,
        ty: &Type,
        derived: &VerifiedDerived,
    ) -> Result<Option<MemoryListWitness>> {
        let Type::List(element) = ty else {
            return Ok(None);
        };
        let id = self
            .memo
            .get(element.as_ref())
            .copied()
            .ok_or_else(|| Error::msg("memory verifier lost list element fact"))?;
        let fact = self.expected(id)?;
        Ok(Some(MemoryListWitness {
            element: fact.witness,
            selected: derived.closure.class == MemoryClosureClass::RegionClosed,
            eligibility: verified_witness_list_element(element, &fact.derived),
            storage: MemoryListStorageKind::SegmentedSessionRegion,
            segment_capacity: 32,
        }))
    }

    fn verified_witness_glue(&self, id: MemoryDropGlueId) -> Result<MemoryDropGlueKind> {
        self.plan
            .drop_glues
            .get(id.index().unwrap_or(usize::MAX))
            .filter(|item| item.id == id)
            .map(|item| item.kind.clone())
            .ok_or_else(|| Error::msg("memory verifier witness glue is missing"))
    }

    fn verified_witness_path(&self, id: MemoryDropPathId) -> Result<Vec<MemoryWitnessDropBranch>> {
        let path = self
            .plan
            .drop_paths
            .get(id.index().unwrap_or(usize::MAX))
            .filter(|item| item.id == id)
            .ok_or_else(|| Error::msg("memory verifier witness path is missing"))?;
        path.branches
            .iter()
            .map(|branch| {
                Ok(MemoryWitnessDropBranch {
                    active_variant: branch.active_variant,
                    actions: branch
                        .actions
                        .iter()
                        .map(|action| {
                            Ok(MemoryWitnessDropAction {
                                path: action.path.clone(),
                                glue: self.verified_witness_glue(action.glue)?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?,
                })
            })
            .collect()
    }
}

include!("witness_policy.rs");
include!("witness_capabilities.rs");
include!("semantic_witness/mod.rs");
