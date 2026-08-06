use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockMetadata {
    pub loop_header: bool,
    pub origin: Origin,
    pub failure_cleanup: Option<FailureCleanupRoots>,
    pub frame_state: Option<FrameState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuredOutcome {
    DeadlineExceeded,
    ResourceLimitExceeded,
    HostFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminator {
    Branch {
        target: BlockId,
        arguments: Vec<ValueId>,
    },
    ConditionalBranch {
        condition: ValueId,
        true_target: BlockId,
        true_arguments: Vec<ValueId>,
        false_target: BlockId,
        false_arguments: Vec<ValueId>,
    },
    Return(ValueId),
    Trap {
        value: ValueId,
    },
    Exit {
        code: ValueId,
    },
    Outcome {
        outcome: StructuredOutcome,
        detail: Option<ValueId>,
    },
}

impl Terminator {
    pub fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::Branch { arguments, .. } => arguments.clone(),
            Self::ConditionalBranch {
                condition,
                true_arguments,
                false_arguments,
                ..
            } => {
                let mut values = Vec::with_capacity(
                    1usize
                        .saturating_add(true_arguments.len())
                        .saturating_add(false_arguments.len()),
                );
                values.push(*condition);
                values.extend(true_arguments);
                values.extend(false_arguments);
                values
            }
            Self::Return(value) | Self::Trap { value } | Self::Exit { code: value } => vec![*value],
            Self::Outcome { detail, .. } => detail.iter().copied().collect(),
        }
    }
}
