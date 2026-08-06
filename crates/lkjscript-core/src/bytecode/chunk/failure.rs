#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UniqueValueKind {
    Bytes,
    ByteVector,
    ByteSlice,
    ByteSliceMut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceReturnKind {
    Resource(crate::ResourceKind),
    Result(crate::ResourceKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureCleanupAction {
    EndBorrow {
        local: usize,
        place: usize,
        kind: UniqueValueKind,
    },
    DropUnique {
        local: usize,
        place: Option<usize>,
        kind: UniqueValueKind,
    },
    DropResource {
        local: usize,
        place: Option<usize>,
        kind: crate::ResourceKind,
    },
    EndStructuralBorrow {
        local: usize,
        place: usize,
        representation: crate::StructuralRepresentationId,
    },
    DropStructural {
        local: usize,
        place: Option<usize>,
        representation: crate::StructuralRepresentationId,
    },
    AbortStructuralDestination {
        local: usize,
        destination: crate::StructuralDestinationId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FailureCleanupPlan {
    pub actions: Vec<FailureCleanupAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureCleanupRange {
    pub start: u16,
    pub end: u16,
    pub plan: Option<u16>,
    pub unentered_plan: Option<u16>,
}
