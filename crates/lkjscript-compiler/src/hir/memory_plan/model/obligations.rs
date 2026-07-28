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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryDropGlueKind {
    ByteVector,
    Resource(ResourceKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryDropGluePlan {
    pub id: MemoryDropGlueId,
    pub kind: MemoryDropGlueKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryObligationKind {
    DropValue,
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
}
