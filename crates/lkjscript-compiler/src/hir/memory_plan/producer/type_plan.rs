#[derive(Clone)]
struct DerivedType {
    mode: MemoryAggregateMode,
    closure: MemoryClosureFact,
    contains_borrow: bool,
    contains_dynamic_owner: bool,
}

struct TypePlanner<'a> {
    program: &'a hir::Program,
    graph: DeclarationGraph,
    memo: HashMap<Type, MemoryTypeFactId>,
    facts: Vec<MemoryTypeFact>,
    witnesses: Vec<MemoryWitness>,
    drop_paths: Vec<MemoryDropPathPlan>,
    glues: Vec<MemoryDropGluePlan>,
    fields: u64,
    variants: u64,
}

impl<'a> TypePlanner<'a> {
    fn new(program: &'a hir::Program) -> Result<Self> {
        Ok(Self {
            program,
            graph: DeclarationGraph::new(program)?,
            memo: HashMap::new(), facts: Vec::new(), witnesses: Vec::new(),
            drop_paths: Vec::new(), glues: base_drop_glues(), fields: 0, variants: 0,
        })
    }

    fn intern(&mut self, ty: &Type) -> Result<MemoryTypeFactId> {
        if let Some(id) = self.memo.get(ty) { return Ok(*id); }
        let mut count = u64::try_from(self.facts.len())
            .map_err(|_| Error::msg("HIR memory-plan type facts exceed u64"))?;
        bounded_add(&mut count, 1, MAX_MEMORY_PLAN_TYPE_NODES, "type nodes")?;
        let derived = self.derive(ty)?;
        if derived.closure.class == MemoryClosureClass::IllegalDomainBridge {
            return Err(closure_error(ty, &derived.closure));
        }
        let id = MemoryTypeFactId::new(u32::try_from(self.facts.len())
            .map_err(|_| Error::msg("HIR memory-plan type fact identity exceeds u32"))?);
        let (drop_glue, drop_path) = self.add_structural_drop(ty, &derived)?;
        let root_projection = if derived.closure.class == MemoryClosureClass::RegionClosed
        {
            MemoryRootProjection::None
        } else if derived.contains_dynamic_owner
            || matches!(ty, Type::Str | Type::Path)
            || (is_aggregate(ty)
                && derived.closure.class == MemoryClosureClass::Deterministic) {
            MemoryRootProjection::Structural
        } else { MemoryRootProjection::None };
        let copy_share = copy_share(ty, &derived);
        let witness = self.add_witness(
            ty, &derived, root_projection, copy_share, drop_glue, drop_path,
        )?;
        self.facts.push(MemoryTypeFact {
            id, witness, ty: memory_type(ty), mode: derived.mode, closure: derived.closure,
            root_projection, copy_share, contains_borrow: derived.contains_borrow,
            contains_dynamic_owner: derived.contains_dynamic_owner, drop_glue, drop_path,
        });
        self.memo.insert(ty.clone(), id);
        Ok(id)
    }

    fn fact(&self, id: MemoryTypeFactId) -> Result<&MemoryTypeFact> {
        id.index().and_then(|index| self.facts.get(index))
            .ok_or_else(|| Error::msg("HIR memory-plan type fact is missing"))
    }

}

include!("type_plan/derive.rs");
include!("type_plan/recursive.rs");
include!("type_plan/witness.rs");
fn closed(class: MemoryClosureClass) -> MemoryClosureFact {
    MemoryClosureFact { class, blocker_path: Vec::new(), blocker_type: None,
        blocker_reason: None, mixed_direction: None }
}

fn unresolved(ty: &Type, reason: MemoryBlockerReason) -> DerivedType {
    DerivedType { mode: MemoryAggregateMode::ImmutableValue,
        closure: MemoryClosureFact { class: MemoryClosureClass::Unresolved,
            blocker_path: Vec::new(), blocker_type: Some(memory_type(ty)),
            blocker_reason: Some(reason), mixed_direction: None },
        contains_borrow: false, contains_dynamic_owner: false }
}
