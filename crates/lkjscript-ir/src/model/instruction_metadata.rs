use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureBehavior {
    None,
    Trap,
    StructuredOutcome,
    TrapOrOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameLocal {
    pub binding: BindingId,
    pub slot: u64,
    pub value: ValueId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameState {
    pub bytecode_position: u64,
    pub locals: Vec<FrameLocal>,
    pub operand_stack: Vec<ValueId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionMetadata {
    pub origin: Origin,
    pub effects: EffectSet,
    pub failure: FailureBehavior,
    pub failure_cleanup: Option<FailureCleanupRoots>,
    pub frame_state: Option<FrameState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureCleanupAction {
    EndBorrow {
        place: PlaceId,
        loan: LoanId,
        kind: BorrowKind,
        value: ValueId,
    },
    DropOwner {
        place: Option<PlaceId>,
        value: ValueId,
        glue: DropGlueIdentity,
    },
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
    ) -> Result<FailureCleanupId> {
        let node = FailureCleanupNode { action, next };
        if let Some(id) = self.ids.get(&node).copied() {
            return Ok(id);
        }
        let raw = u64::try_from(self.nodes.len())
            .map_err(|_| IrError::new("SSA failure-cleanup node count exceeds u64"))?;
        self.nodes
            .try_reserve(1)
            .map_err(|_| IrError::new("SSA failure-cleanup node reservation failed"))?;
        self.ids
            .try_reserve(1)
            .map_err(|_| IrError::new("SSA failure-cleanup interner reservation failed"))?;
        let id = FailureCleanupId::new(raw);
        self.nodes.push(node);
        self.ids.insert(node, id);
        Ok(id)
    }

    pub fn intern_chain<I>(&mut self, actions: I) -> Result<Option<FailureCleanupId>>
    where
        I: IntoIterator<Item = FailureCleanupAction>,
        I::IntoIter: DoubleEndedIterator,
    {
        let mut root = None;
        for action in actions.into_iter().rev() {
            root = Some(self.intern(action, root)?);
        }
        Ok(root)
    }

    #[must_use]
    pub fn into_nodes(self) -> Vec<FailureCleanupNode> {
        self.nodes
    }
}
