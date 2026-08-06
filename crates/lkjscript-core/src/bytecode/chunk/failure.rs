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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FailureCleanupId(u64);

impl FailureCleanupId {
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn index(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FailureCleanupRoots {
    pub loans: Option<FailureCleanupId>,
    pub unplaced: Option<FailureCleanupId>,
    pub places: Option<FailureCleanupId>,
}

impl FailureCleanupRoots {
    #[must_use]
    pub const fn single(root: FailureCleanupId) -> Self {
        Self {
            loans: None,
            unplaced: Some(root),
            places: None,
        }
    }

    pub fn ids(self) -> impl Iterator<Item = FailureCleanupId> {
        [self.loans, self.unplaced, self.places]
            .into_iter()
            .flatten()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FailureCleanupNode {
    pub action: FailureCleanupAction,
    pub next: Option<FailureCleanupId>,
}

#[derive(Debug, Default)]
pub struct FailureCleanupInterner {
    nodes: Vec<FailureCleanupNode>,
    ids: std::collections::HashMap<FailureCleanupNode, FailureCleanupId>,
}

impl FailureCleanupInterner {
    #[must_use]
    pub fn nodes(&self) -> &[FailureCleanupNode] {
        &self.nodes
    }

    pub fn intern(
        &mut self,
        action: FailureCleanupAction,
        next: Option<FailureCleanupId>,
    ) -> crate::Result<FailureCleanupId> {
        let node = FailureCleanupNode { action, next };
        if let Some(id) = self.ids.get(&node).copied() {
            return Ok(id);
        }
        let raw = u64::try_from(self.nodes.len())
            .map_err(|_| crate::Error::msg("bytecode failure-cleanup node count exceeds u64"))?;
        self.nodes
            .try_reserve(1)
            .map_err(|_| crate::Error::host("bytecode cleanup node reservation failed"))?;
        self.ids
            .try_reserve(1)
            .map_err(|_| crate::Error::host("bytecode cleanup interner reservation failed"))?;
        let id = FailureCleanupId::new(raw);
        self.nodes.push(node);
        self.ids.insert(node, id);
        Ok(id)
    }

    #[must_use]
    pub fn into_nodes(self) -> Vec<FailureCleanupNode> {
        self.nodes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureCleanupRange {
    pub start: u64,
    pub end: u64,
    pub plan: Option<FailureCleanupRoots>,
    pub unentered_plan: Option<FailureCleanupId>,
}
