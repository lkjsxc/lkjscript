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
        local: u8,
        place: u8,
        kind: UniqueValueKind,
    },
    DropUnique {
        local: u8,
        place: Option<u8>,
        kind: UniqueValueKind,
    },
    DropResource {
        local: u8,
        place: Option<u8>,
        kind: crate::ResourceKind,
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
