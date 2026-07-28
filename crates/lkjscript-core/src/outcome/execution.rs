use super::{CleanupFailures, HostError, OwnedValue, ResourceLimitKind, Trap};

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionOutcome {
    Returned(OwnedValue),
    Exited(i32),
    Trapped(Trap),
    DeadlineExceeded,
    ResourceLimitExceeded(ResourceLimitKind),
    HostFailure(HostError),
    CleanupFailed {
        primary: Box<ExecutionOutcome>,
        failures: CleanupFailures,
    },
}

impl ExecutionOutcome {
    pub fn returned(&self) -> Option<&OwnedValue> {
        match self.primary() {
            Self::Returned(value) => Some(value),
            _ => None,
        }
    }

    pub fn with_cleanup_failures(self, failures: CleanupFailures) -> Self {
        if failures.is_empty() {
            self
        } else {
            Self::CleanupFailed {
                primary: Box::new(self),
                failures,
            }
        }
    }

    pub fn primary(&self) -> &Self {
        match self {
            Self::CleanupFailed { primary, .. } => primary.primary(),
            outcome => outcome,
        }
    }

    pub fn cleanup_failures(&self) -> Option<&CleanupFailures> {
        match self {
            Self::CleanupFailed { failures, .. } => Some(failures),
            _ => None,
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Self::Returned(value) => format!("Returned({value:?})"),
            Self::Exited(code) => format!("Exited({code})"),
            Self::Trapped(trap) => format!("Trapped({trap})"),
            Self::DeadlineExceeded => "DeadlineExceeded".to_string(),
            Self::ResourceLimitExceeded(kind) => {
                format!("ResourceLimitExceeded({kind:?})")
            }
            Self::HostFailure(error) => format!("HostFailure({error})"),
            Self::CleanupFailed { primary, failures } => format!(
                "CleanupFailed(primary={}, retained={}, omitted={})",
                primary.summary(),
                failures.retained().len(),
                failures.omitted_failures()
            ),
        }
    }
}
