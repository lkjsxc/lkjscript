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
    product_indices: HashMap<String, usize>,
    enum_indices: HashMap<[u8; 32], usize>,
    memo: HashMap<Type, MemoryTypeFactId>,
    facts: Vec<MemoryTypeFact>,
    witnesses: Vec<MemoryWitness>,
    witness_indices: HashMap<MemoryWitnessId, usize>,
    drop_paths: Vec<MemoryDropPathPlan>,
    glues: Vec<MemoryDropGluePlan>,
    fields: u64,
    variants: u64,
}

impl<'a> TypePlanner<'a> {
    fn new(program: &'a hir::Program) -> Result<Self> {
        let mut product_indices = HashMap::new();
        product_indices
            .try_reserve(program.products.len())
            .map_err(|_| Error::host("HIR memory-plan product-name index allocation failed"))?;
        for (index, product) in program.products.iter().enumerate() {
            if product_indices.insert(product.name.clone(), index).is_some() {
                return Err(Error::msg("HIR memory-plan product names are not unique"));
            }
        }
        let mut enum_indices = HashMap::new();
        enum_indices
            .try_reserve(program.enums.len())
            .map_err(|_| Error::host("HIR memory-plan enum index allocation failed"))?;
        for (index, enumeration) in program.enums.iter().enumerate() {
            if enum_indices
                .insert(enumeration.id.bytes(), index)
                .is_some()
            {
                return Err(Error::msg("HIR memory-plan enum identities are not unique"));
            }
        }
        Ok(Self {
            program,
            graph: DeclarationGraph::new(program)?,
            product_indices,
            enum_indices,
            memo: HashMap::new(), facts: Vec::new(), witnesses: Vec::new(),
            witness_indices: HashMap::new(), drop_paths: Vec::new(),
            glues: base_drop_glues(), fields: 0, variants: 0,
        })
    }

    fn intern(&mut self, ty: &Type) -> Result<MemoryTypeFactId> {
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
                .map_err(|_| Error::msg("HIR memory-plan type work allocation failed"))?;
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
            .ok_or_else(|| Error::msg("HIR memory-plan list interning omitted its root"))
    }

    fn intern_inner(&mut self, ty: &Type) -> Result<MemoryTypeFactId> {
        if let Some(id) = self.memo.get(ty) { return Ok(*id); }
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

    fn product(&self, name: &str) -> Result<&hir::ProductDefinition> {
        self.product_indices
            .get(name)
            .and_then(|index| self.program.products.get(*index))
            .filter(|product| product.name == name)
            .ok_or_else(|| Error::msg(format!("HIR memory plan references unknown product {name}")))
    }

    fn enumeration(&self, id: [u8; 32]) -> Result<&hir::EnumDefinition> {
        self.enum_indices
            .get(&id)
            .and_then(|index| self.program.enums.get(*index))
            .filter(|enumeration| enumeration.id.bytes() == id)
            .ok_or_else(|| Error::msg("HIR memory plan references unknown enum"))
    }

    fn fact(&self, id: MemoryTypeFactId) -> Result<&MemoryTypeFact> {
        id.index().and_then(|index| self.facts.get(index))
            .ok_or_else(|| Error::msg("HIR memory-plan type fact is missing"))
    }

}

include!("derive.rs");
include!("recursive.rs");
include!("witness.rs");
include!("group.rs");
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
