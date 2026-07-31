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
    RegisteredLegacyTraced,
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
    LegacyClosed,
    IllegalMixedBridge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryBlockerReason {
    RegisteredLegacyValue,
    RecursiveDeclarationScc,
    UnknownTypeParameter,
    ListPair,
    CapturedClosure,
    DynamicDeterministicOwner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryMixedBridgeDirection {
    LegacyContainsDeterministic,
    DeterministicContainsLegacy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryTypePathElement {
    ProductField { index: u32, name: String },
    EnumVariantField {
        variant_index: u32,
        variant: [u8; 32],
        field_index: u32,
        field: [u8; 32],
    },
    TypeArgument(u32),
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
    LegacyTracing,
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
    RegisteredLegacyTraced,
    CutoverRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryDestinationField {
    pub index: u32,
    pub expression: MemoryExpressionId,
    pub drop_path: Option<MemoryDropPathId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryActivePayload {
    pub variant: [u8; 32],
    pub source_order: u16,
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
    pub field_count: u32,
    pub fields: Vec<MemoryDestinationField>,
    pub active_payload: Option<MemoryActivePayload>,
    pub initialized_order: Vec<u32>,
    pub reverse_abort_cleanup: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryBorrowScopePlan {
    pub id: MemoryBorrowScopeId,
    pub function: MemoryFunctionId,
    pub call: MemoryCallId,
    pub argument_index: u32,
    pub source_expression: MemoryExpressionId,
    pub binding: u32,
    pub place: u32,
    pub kind: MemoryBorrowKind,
    pub semantic_uses: u32,
    pub end_after: MemoryExpressionId,
}
