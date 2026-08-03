use super::*;

#[derive(Clone)]
pub(crate) struct VerifiedDerived {
    pub(crate) mode: MemoryAggregateMode,
    pub(crate) closure: MemoryClosureFact,
    pub(crate) contains_borrow: bool,
    pub(crate) contains_dynamic_owner: bool,
}

#[derive(Clone)]
pub(crate) struct VerifiedExpectedType {
    pub(crate) witness: MemoryWitnessId,
    pub(crate) derived: VerifiedDerived,
    pub(crate) glue: Option<MemoryDropGlueId>,
    pub(crate) path: Option<MemoryDropPathId>,
}

pub(crate) struct VerifiedTypes<'a> {
    pub(crate) program: &'a hir::Program,
    pub(crate) plan: &'a HirMemoryPlan,
    pub(crate) graph: VerifiedDeclarationGraph,
    pub(crate) memo: HashMap<Type, MemoryTypeFactId>,
    pub(crate) expected: Vec<VerifiedExpectedType>,
    pub(crate) fields: u64,
    pub(crate) variants: u64,
}

impl<'a> VerifiedTypes<'a> {
    pub(crate) fn new(program: &'a hir::Program, plan: &'a HirMemoryPlan) -> Result<Self> {
        Ok(Self {
            program,
            plan,
            graph: VerifiedDeclarationGraph::new(program)?,
            memo: HashMap::new(),
            expected: Vec::new(),
            fields: 0,
            variants: 0,
        })
    }

    pub(crate) fn intern(&mut self, ty: &Type) -> Result<MemoryTypeFactId> {
        if let Some(id) = self.memo.get(ty) {
            return Ok(*id);
        }
        if u64::try_from(self.expected.len()).unwrap_or(u64::MAX) >= MAX_MEMORY_PLAN_TYPE_NODES {
            return Err(Error::msg(
                "memory verifier type facts exceed bounded maximum",
            ));
        }
        let derived = self.derive(ty)?;
        if derived.closure.class == MemoryClosureClass::IllegalDomainBridge {
            return Err(Error::msg(
                "independent memory verifier reconstructed an illegal mixed bridge",
            ));
        }
        let id = MemoryTypeFactId::new(index_u32(self.expected.len())?);
        let (glue, path) = self.expected_drop(ty, &derived)?;
        let fact = self
            .plan
            .type_facts
            .get(id.index().unwrap_or(usize::MAX))
            .ok_or_else(|| Error::msg("HIR memory plan omitted a reconstructed type fact"))?;
        let root = if derived.closure.class == MemoryClosureClass::RegionClosed {
            MemoryRootProjection::None
        } else if derived.contains_dynamic_owner
            || matches!(ty, Type::Str | Type::Path)
            || (verified_is_aggregate(ty)
                && derived.closure.class == MemoryClosureClass::Deterministic)
        {
            MemoryRootProjection::Structural
        } else {
            MemoryRootProjection::None
        };
        let copy_share = verified_copy_share(ty, &derived);
        let witness = self.verify_witness(ty, &derived, root, copy_share, glue, path)?;
        if fact.id != id
            || fact.witness != witness
            || !type_matches(ty, &fact.ty)
            || fact.mode != derived.mode
            || fact.closure != derived.closure
            || fact.root_projection != root
            || fact.copy_share != copy_share
            || fact.contains_borrow != derived.contains_borrow
            || fact.contains_dynamic_owner != derived.contains_dynamic_owner
            || fact.drop_glue != glue
            || fact.drop_path != path
        {
            return Err(Error::msg(
                "independent HIR memory verifier rejected type authority fact",
            ));
        }
        self.expected.push(VerifiedExpectedType {
            witness,
            derived,
            glue,
            path,
        });
        self.memo.insert(ty.clone(), id);
        Ok(id)
    }

    pub(crate) fn expected(&self, id: MemoryTypeFactId) -> Result<&VerifiedExpectedType> {
        id.index()
            .and_then(|index| self.expected.get(index))
            .ok_or_else(|| Error::msg("memory verifier expected type is missing"))
    }

    pub(crate) fn verify_totals(&mut self) -> Result<()> {
        self.close_recursive_members()?;
        verify_witness_groups(self.plan)?;
        let type_nodes = u64::try_from(self.expected.len()).unwrap_or(u64::MAX);
        let drop_paths = u64::try_from(
            self.expected
                .iter()
                .filter(|item| item.path.is_some())
                .count(),
        )
        .unwrap_or(u64::MAX);
        let witnesses = u64::try_from(self.plan.witnesses.len()).unwrap_or(u64::MAX);
        let unique_witnesses: BTreeSet<_> =
            self.plan.witnesses.iter().map(|item| item.id).collect();
        let group_edges = self
            .plan
            .witnesses
            .iter()
            .try_fold(0u64, |sum, witness| {
                sum.checked_add(u64::try_from(witness.facts.dependencies.len()).ok()?)
            })
            .unwrap_or(u64::MAX);
        if type_nodes > MAX_MEMORY_PLAN_TYPE_NODES
            || witnesses > MAX_MEMORY_PLAN_WITNESSES
            || witnesses != type_nodes
            || unique_witnesses.len() != self.plan.witnesses.len()
            || drop_paths > MAX_MEMORY_PLAN_DROP_PATHS
            || self.plan.type_facts.len() != self.expected.len()
            || self.plan.drop_paths.len() != usize::try_from(drop_paths).unwrap_or(usize::MAX)
            || self.plan.drop_glues.len()
                != ResourceKind::ALL
                    .len()
                    .saturating_add(2)
                    .saturating_add(usize::try_from(drop_paths).unwrap_or(usize::MAX))
            || self.plan.work.type_nodes != type_nodes
            || self.plan.work.witnesses != witnesses
            || self.plan.work.witness_groups
                != u64::try_from(self.plan.witness_groups.len()).unwrap_or(u64::MAX)
            || self.plan.work.witness_group_edges != group_edges
            || self.plan.work.type_edges != self.graph.edges
            || self.plan.work.scc_work != self.graph.scc_work
            || self.plan.work.aggregate_fields != self.fields
            || self.plan.work.aggregate_variants != self.variants
            || self.plan.work.drop_paths != drop_paths
        {
            return Err(Error::msg(
                "independent memory verifier rejected bounded type work/tables",
            ));
        }
        Ok(())
    }
}
