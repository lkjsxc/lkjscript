use super::{EvalValue, RuntimeOp};

#[derive(Debug, PartialEq)]
pub enum EvalOutcome {
    Returned(EvalValue),
    Exited(i64),
    Trapped(String),
    UnsupportedOperation(RuntimeOp),
    DeadlineExceeded,
    ResourceLimitExceeded(String),
    HostFailure(String),
    CleanupFailed {
        primary: Box<EvalOutcome>,
        failures: lkjscript_core::CleanupFailures,
    },
}

impl EvalOutcome {
    pub fn primary(&self) -> &Self {
        match self {
            Self::CleanupFailed { primary, .. } => primary.primary(),
            outcome => outcome,
        }
    }

    pub fn cleanup_failures(&self) -> Option<&lkjscript_core::CleanupFailures> {
        match self {
            Self::CleanupFailed { failures, .. } => Some(failures),
            _ => None,
        }
    }

    pub(super) fn with_cleanup_failures(self, failures: lkjscript_core::CleanupFailures) -> Self {
        if failures.is_empty() {
            self
        } else {
            Self::CleanupFailed {
                primary: Box::new(self),
                failures,
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum Flow {
    Exit(i64),
    Trap(String),
    Unsupported(RuntimeOp),
    Deadline,
    Resource(String),
    HostFailure(String),
}

impl Flow {
    pub(super) fn outcome(self) -> EvalOutcome {
        match self {
            Self::Exit(code) => EvalOutcome::Exited(code),
            Self::Trap(message) => EvalOutcome::Trapped(message),
            Self::Unsupported(operation) => EvalOutcome::UnsupportedOperation(operation),
            Self::Deadline => EvalOutcome::DeadlineExceeded,
            Self::Resource(kind) => EvalOutcome::ResourceLimitExceeded(kind),
            Self::HostFailure(message) => EvalOutcome::HostFailure(message),
        }
    }

    pub(super) fn detail(self) -> String {
        match self.outcome() {
            EvalOutcome::HostFailure(message)
            | EvalOutcome::Trapped(message)
            | EvalOutcome::ResourceLimitExceeded(message) => message,
            outcome => format!("{outcome:?}"),
        }
    }
}
