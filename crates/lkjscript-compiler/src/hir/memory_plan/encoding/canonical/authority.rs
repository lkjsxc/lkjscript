use super::*;

unit_enum!(MemoryDomain {
    Inline = 0,
    Static = 1,
    Stack = 2,
    CallerDestination = 3,
    UniqueStructural = 4,
    OrdinaryRegion = 5,
    SealedRegion = 6,
    BorrowedView = 7,
    ExternalResource = 8,
    UnsupportedRuntime = 9,
});
unit_enum!(MemoryRootProjection { None = 0, Structural = 1 });
unit_enum!(MemoryAggregateMode { Copy = 0, ImmutableValue = 1, Affine = 2 });
unit_enum!(MemoryClosureClass {
    Deterministic = 0,
    RegionClosed = 1,
    Unresolved = 2,
    IllegalDomainBridge = 3,
});
unit_enum!(MemoryBlockerReason {
    UnsupportedRuntimeValue = 0,
    UnknownTypeParameter = 1,
    ListElementWitnessRequired = 2,
    RegionDomainBoundary = 3,
    CapturedClosure = 4,
    DynamicDeterministicOwner = 5,
});
unit_enum!(MemoryMixedBridgeDirection {
    UnresolvedContainsDeterministic = 0,
    DeterministicContainsUnresolved = 1,
});
unit_enum!(MemoryCopySharePlan {
    TrivialCopy = 0,
    StaticIdentity = 1,
    StructuralCopy = 2,
    BorrowShared = 3,
    BorrowExclusive = 4,
    Move = 5,
    SealedShare = 6,
    RegionHandleCopy = 7,
    Unsupported = 8,
    ExternalHandle = 9,
});
unit_enum!(MemoryExecution { Current = 0, CutoverRequired = 1 });
unit_enum!(MemoryDestinationKind {
    Stack = 0,
    CallerDestination = 1,
    UniqueStructural = 2,
    OrdinaryRegion = 3,
    SealedRegion = 4,
    UnsupportedRuntime = 5,
    CutoverRequired = 6,
});

impl Canonical for MemoryTypePathElement {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        match self {
            Self::ProductField { index, field } => {
                output.tag(0)?;
                output.value(index)?;
                output.value(field)
            }
            Self::EnumVariantField {
                variant_index,
                variant,
                field_index,
                field,
            } => {
                output.tag(1)?;
                output.value(variant_index)?;
                output.value(variant)?;
                output.value(field_index)?;
                output.value(field)
            }
            Self::TypeArgument(index) => {
                output.tag(2)?;
                output.value(index)
            }
        }
    }
}

impl Canonical for MemoryExecutionCutover {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        match self {
            Self::StructuralString => output.tag(0),
            Self::StructuralPath => output.tag(1),
            Self::Product(id) => {
                output.tag(2)?;
                output.value(&id.raw())
            }
            Self::Enum { id, arguments } => {
                output.tag(3)?;
                output.value(id)?;
                output.value(arguments)
            }
        }
    }
}

canonical_struct!(MemoryClosureFact {
    class,
    blocker_path,
    blocker_type,
    blocker_reason,
    mixed_direction,
});
canonical_struct!(MemoryTypeFact {
    id,
    witness,
    ty,
    mode,
    closure,
    root_projection,
    copy_share,
    contains_borrow,
    contains_dynamic_owner,
    drop_glue,
    drop_path,
});
canonical_struct!(MemoryDestinationField {
    index,
    expression,
    drop_path
});
canonical_struct!(MemoryActivePayload {
    variant,
    source_order
});
canonical_struct!(MemoryDestinationPlan {
    id,
    function,
    expression,
    kind,
    execution,
    execution_cutover,
    type_fact,
    field_count,
    fields,
    active_payload,
    initialized_order,
    reverse_abort_cleanup,
});
canonical_struct!(MemoryBorrowScopePlan {
    id,
    function,
    call,
    argument_index,
    source_expression,
    binding,
    place,
    kind,
    semantic_uses,
    end_after,
});
