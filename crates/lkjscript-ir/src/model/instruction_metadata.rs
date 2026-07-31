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
    pub slot: u16,
    pub value: ValueId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameState {
    pub bytecode_position: u32,
    pub locals: Vec<FrameLocal>,
    pub operand_stack: Vec<ValueId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionMetadata {
    pub origin: Origin,
    pub effects: EffectSet,
    pub failure: FailureBehavior,
    pub failure_cleanup: Option<FailureCleanupId>,
    pub frame_state: Option<FrameState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureCleanupPlan {
    pub id: FailureCleanupId,
    pub actions: Vec<FailureCleanupAction>,
}
