#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryDomain {
    Inline,
    Static,
    Stack,
    CallerDestination,
    UniqueStructural,
    OrdinaryRegion,
    SealedRegion,
    BorrowedView,
    ExternalResource,
    UnsupportedRuntime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryRootProjection {
    None,
    Structural,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MemoryAggregateMode {
    Copy,
    ImmutableValue,
    Affine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryClosureClass {
    Deterministic,
    RegionClosed,
    Unresolved,
    IllegalDomainBridge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryBlockerReason {
    UnsupportedRuntimeValue,
    UnknownTypeParameter,
    ListElementWitnessRequired,
    RegionDomainBoundary,
    CapturedClosure,
    DynamicDeterministicOwner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryMixedBridgeDirection {
    UnresolvedContainsDeterministic,
    DeterministicContainsUnresolved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryTypePathElement {
    ProductField { index: u64, name: String },
    EnumVariantField {
        variant_index: u64,
        variant: [u8; 32],
        field_index: u64,
        field: [u8; 32],
    },
    TypeArgument(u64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryClosureFact {
    pub class: MemoryClosureClass,
    pub blocker_path: Vec<MemoryTypePathElement>,
    pub blocker_type: Option<MemoryType>,
    pub blocker_reason: Option<MemoryBlockerReason>,
    pub mixed_direction: Option<MemoryMixedBridgeDirection>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryCopySharePlan {
    TrivialCopy,
    StaticIdentity,
    StructuralCopy,
    BorrowShared,
    BorrowExclusive,
    Move,
    SealedShare,
    RegionHandleCopy,
    Unsupported,
    ExternalHandle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryExecution {
    Current,
    CutoverRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryExecutionCutover {
    StructuralString,
    StructuralPath,
    Product(String),
    Enum { id: [u8; 32], arguments: Vec<MemoryType> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryTypeFact {
    pub id: MemoryTypeFactId,
    pub witness: MemoryWitnessId,
    pub ty: MemoryType,
    pub mode: MemoryAggregateMode,
    pub closure: MemoryClosureFact,
    pub root_projection: MemoryRootProjection,
    pub copy_share: MemoryCopySharePlan,
    pub contains_borrow: bool,
    pub contains_dynamic_owner: bool,
    pub drop_glue: Option<MemoryDropGlueId>,
    pub drop_path: Option<MemoryDropPathId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryDestinationKind {
    Stack,
    CallerDestination,
    UniqueStructural,
    OrdinaryRegion,
    SealedRegion,
    UnsupportedRuntime,
    CutoverRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryDestinationField {
    pub index: u64,
    pub expression: MemoryExpressionId,
    pub drop_path: Option<MemoryDropPathId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryActivePayload {
    pub variant: [u8; 32],
    pub source_order: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryDestinationPlan {
    pub id: MemoryDestinationId,
    pub function: MemoryFunctionId,
    pub expression: MemoryExpressionId,
    pub kind: MemoryDestinationKind,
    pub execution: MemoryExecution,
    pub execution_cutover: Option<MemoryExecutionCutover>,
    pub type_fact: MemoryTypeFactId,
    pub field_count: u64,
    pub fields: Vec<MemoryDestinationField>,
    pub active_payload: Option<MemoryActivePayload>,
    pub initialized_order: Vec<u64>,
    pub reverse_abort_cleanup: Vec<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryBorrowScopePlan {
    pub id: MemoryBorrowScopeId,
    pub function: MemoryFunctionId,
    pub call: MemoryCallId,
    pub argument_index: u64,
    pub source_expression: MemoryExpressionId,
    pub binding: u64,
    pub place: u64,
    pub kind: MemoryBorrowKind,
    pub semantic_uses: u64,
    pub end_after: MemoryExpressionId,
}
