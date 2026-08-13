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
    pub(crate) product_indices: HashMap<hir::ProductId, usize>,
    pub(crate) enum_indices: HashMap<[u8; 32], usize>,
    pub(crate) memo: HashMap<Type, MemoryTypeFactId>,
    pub(crate) expected: Vec<VerifiedExpectedType>,
    pub(crate) fields: u64,
    pub(crate) variants: u64,
    pub(crate) drop_paths: u64,
}

impl<'a> VerifiedTypes<'a> {
    pub(crate) fn new(program: &'a hir::Program, plan: &'a HirMemoryPlan) -> Result<Self> {
        let mut product_indices = HashMap::new();
        product_indices
            .try_reserve(program.products.len())
            .map_err(|_| Error::host("memory verifier product index allocation failed"))?;
        for (index, product) in program.products.iter().enumerate() {
            if product_indices.insert(product.id, index).is_some() {
                return Err(Error::msg(
                    "memory verifier product identities are not unique",
                ));
            }
        }
        let mut enum_indices = HashMap::new();
        enum_indices
            .try_reserve(program.enums.len())
            .map_err(|_| Error::host("memory verifier enum index allocation failed"))?;
        for (index, enumeration) in program.enums.iter().enumerate() {
            if enum_indices.insert(enumeration.id.bytes(), index).is_some() {
                return Err(Error::msg("memory verifier enum identities are not unique"));
            }
        }
        Ok(Self {
            program,
            plan,
            graph: VerifiedDeclarationGraph::new(program)?,
            product_indices,
            enum_indices,
            memo: HashMap::new(),
            expected: Vec::new(),
            fields: 0,
            variants: 0,
            drop_paths: 0,
        })
    }

    pub(crate) fn intern(&mut self, ty: &Type) -> Result<MemoryTypeFactId> {
        if let Some(id) = self.memo.get(ty) {
            return Ok(*id);
        }
        let mut lists = Vec::new();
        let mut current = ty;
        while let Type::List(inner) = current {
            if self.memo.contains_key(current) {
                break;
            }
            lists
                .try_reserve(1)
                .map_err(|_| Error::msg("memory verifier type work allocation failed"))?;
            lists.push(current);
            current = inner;
        }
        if lists.is_empty() {
            return crate::stack::grow(|| self.intern_inner(ty));
        }
        self.intern(current)?;
        for list in lists.into_iter().rev() {
            self.intern_inner(list)?;
        }
        self.memo
            .get(ty)
            .copied()
            .ok_or_else(|| Error::msg("memory verifier list interning omitted its root"))
    }

    fn intern_inner(&mut self, ty: &Type) -> Result<MemoryTypeFactId> {
        if let Some(id) = self.memo.get(ty) {
            return Ok(*id);
        }
        let derived = self.derive(ty)?;
        if derived.closure.class == MemoryClosureClass::IllegalDomainBridge {
            return Err(Error::msg(
                "independent memory verifier reconstructed an illegal mixed bridge",
            ));
        }
        let id = MemoryTypeFactId::new(index_u64(self.expected.len())?);
        let (glue, path) = self.expected_drop(ty, &derived)?;
        let fact = id
            .index()
            .and_then(|index| self.plan.type_facts.get(index))
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

    pub(crate) fn product_definition(&self, id: hir::ProductId) -> Result<&hir::ProductDefinition> {
        self.product_indices
            .get(&id)
            .and_then(|index| self.program.products.get(*index))
            .filter(|product| product.id == id)
            .ok_or_else(|| Error::msg("memory verifier lost product"))
    }

    pub(crate) fn enum_definition(&self, id: [u8; 32]) -> Result<&hir::EnumDefinition> {
        self.enum_indices
            .get(&id)
            .and_then(|index| self.program.enums.get(*index))
            .filter(|enumeration| enumeration.id.bytes() == id)
            .ok_or_else(|| Error::msg("memory verifier lost enum"))
    }

    pub(crate) fn expected(&self, id: MemoryTypeFactId) -> Result<&VerifiedExpectedType> {
        id.index()
            .and_then(|index| self.expected.get(index))
            .ok_or_else(|| Error::msg("memory verifier expected type is missing"))
    }

    pub(crate) fn verify_totals(&mut self) -> Result<()> {
        self.close_recursive_members()?;
        verify_witness_groups(self.plan)?;
        let type_nodes = u64::try_from(self.expected.len())
            .map_err(|_| Error::msg("memory verifier type count exceeds u64"))?;
        let witnesses = u64::try_from(self.plan.witnesses.len())
            .map_err(|_| Error::msg("memory verifier witness count exceeds u64"))?;
        let witness_groups = u64::try_from(self.plan.witness_groups.len())
            .map_err(|_| Error::msg("memory verifier witness-group count exceeds u64"))?;
        let unique_witnesses: BTreeSet<_> =
            self.plan.witnesses.iter().map(|item| item.id).collect();
        let group_edges = self.plan.witnesses.iter().try_fold(0u64, |sum, witness| {
            let dependencies = u64::try_from(witness.facts.dependencies.len())
                .map_err(|_| Error::msg("memory witness dependency count exceeds u64"))?;
            sum.checked_add(dependencies)
                .ok_or_else(|| Error::msg("memory witness dependency count overflow"))
        })?;
        let drop_paths = usize::try_from(self.drop_paths)
            .map_err(|_| Error::msg("memory verifier drop-path count exceeds host usize"))?;
        let expected_glues = ResourceKind::ALL
            .len()
            .checked_add(2)
            .and_then(|count| count.checked_add(drop_paths))
            .ok_or_else(|| Error::msg("memory verifier drop-glue count overflow"))?;
        if witnesses != type_nodes
            || unique_witnesses.len() != self.plan.witnesses.len()
            || self.plan.type_facts.len() != self.expected.len()
            || self.plan.drop_paths.len() != drop_paths
            || self.plan.drop_glues.len() != expected_glues
            || self.plan.work.type_nodes != type_nodes
            || self.plan.work.witnesses != witnesses
            || self.plan.work.witness_groups != witness_groups
            || self.plan.work.witness_group_edges != group_edges
            || self.plan.work.type_edges != self.graph.edges
            || self.plan.work.scc_work != self.graph.scc_work
            || self.plan.work.aggregate_fields != self.fields
            || self.plan.work.aggregate_variants != self.variants
            || self.plan.work.drop_paths != self.drop_paths
        {
            return Err(Error::msg(
                "independent memory verifier rejected exact type work/tables",
            ));
        }
        Ok(())
    }
}
