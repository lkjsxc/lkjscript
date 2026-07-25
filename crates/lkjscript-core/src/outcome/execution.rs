use super::{HostError, OwnedValue, ResourceLimitKind, Trap};

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionOutcome {
    Returned(OwnedValue),
    Exited(i32),
    Trapped(Trap),
    DeadlineExceeded,
    ResourceLimitExceeded(ResourceLimitKind),
    HostFailure(HostError),
}

impl ExecutionOutcome {
    pub fn returned(&self) -> Option<&OwnedValue> {
        match self {
            Self::Returned(value) => Some(value),
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
        }
    }
}
