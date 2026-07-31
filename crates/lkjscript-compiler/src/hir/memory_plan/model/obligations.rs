#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryLoanPlan {
    pub function: MemoryFunctionId,
    pub place: u32,
    pub loan: u32,
    pub expression: MemoryExpressionId,
    pub binding: Option<u32>,
    pub kind: MemoryBorrowKind,
    pub semantic_uses: u32,
    pub end_after: MemoryExpressionId,
    pub entry: MemoryEntryId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryDropGlueKind {
    ByteVector,
    Bytes,
    Resource(ResourceKind),
    String,
    Path,
    Product(String),
    Enum { id: [u8; 32], arguments: Vec<MemoryType> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryDropGluePlan {
    pub id: MemoryDropGlueId,
    pub kind: MemoryDropGlueKind,
    pub drop_path: Option<MemoryDropPathId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryDropPathElement {
    ProductField { index: u32, name: String },
    EnumField {
        variant: [u8; 32],
        index: u32,
        field: [u8; 32],
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryDropAction {
    pub path: Vec<MemoryDropPathElement>,
    pub glue: MemoryDropGlueId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryDropBranch {
    pub active_variant: Option<[u8; 32]>,
    pub actions: Vec<MemoryDropAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryDropPathPlan {
    pub id: MemoryDropPathId,
    pub ty: MemoryType,
    pub branches: Vec<MemoryDropBranch>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryDropClass {
    Static,
    Dead,
    Conditional,
    Open,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryObligationKind {
    DropWholeValue,
    DropResource(ResourceKind),
    EndBorrow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryObligation {
    pub id: MemoryObligationId,
    pub function: MemoryFunctionId,
    pub entry: MemoryEntryId,
    pub kind: MemoryObligationKind,
    pub drop_glue: Option<MemoryDropGlueId>,
    pub drop_path: Option<MemoryDropPathId>,
    pub drop_class: Option<MemoryDropClass>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MemoryPlanWork {
    pub functions: u64,
    pub entries: u64,
    pub expressions: u64,
    pub uses: u64,
    pub loans: u64,
    pub constants: u64,
    pub calls: u64,
    pub obligations: u64,
    pub type_nodes: u64,
    pub type_edges: u64,
    pub scc_work: u64,
    pub aggregate_fields: u64,
    pub aggregate_variants: u64,
    pub destinations: u64,
    pub borrow_scopes: u64,
    pub drop_paths: u64,
    pub verifier_steps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirMemoryPlan {
    pub schema: &'static str,
    pub id: MemoryPlanId,
    pub functions: Vec<FunctionMemoryPlan>,
    pub entries: Vec<MemoryPlanEntry>,
    pub uses: Vec<MemoryUse>,
    pub loans: Vec<MemoryLoanPlan>,
    pub constants: Vec<MemoryConstantPlan>,
    pub calls: Vec<MemoryCallPlan>,
    pub obligations: Vec<MemoryObligation>,
    pub type_facts: Vec<MemoryTypeFact>,
    pub destinations: Vec<MemoryDestinationPlan>,
    pub borrow_scopes: Vec<MemoryBorrowScopePlan>,
    pub drop_paths: Vec<MemoryDropPathPlan>,
    pub drop_glues: Vec<MemoryDropGluePlan>,
    pub work: MemoryPlanWork,
}

impl HirMemoryPlan {
    pub fn entry(&self, id: MemoryEntryId) -> Option<&MemoryPlanEntry> {
        id.index().and_then(|index| self.entries.get(index))
    }

    pub fn function(&self, id: MemoryFunctionId) -> Option<&FunctionMemoryPlan> {
        id.index().and_then(|index| self.functions.get(index))
    }

    pub fn type_fact(&self, id: MemoryTypeFactId) -> Option<&MemoryTypeFact> {
        id.index().and_then(|index| self.type_facts.get(index))
    }

    pub fn destination(&self, id: MemoryDestinationId) -> Option<&MemoryDestinationPlan> {
        id.index().and_then(|index| self.destinations.get(index))
    }
}
