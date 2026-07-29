use std::fmt;

use lkjscript_host::HostError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlStoreError {
    Host(HostError),
    ContractUnavailable,
    ContractMismatch,
    StaleRevision { found: u64 },
    Corrupt(&'static str),
    Limit(&'static str),
    InvalidKey,
    SequenceExhausted,
}

impl fmt::Display for ControlStoreError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(error) => write!(output, "control storage: {error}"),
            Self::ContractUnavailable => output.write_str("runtime control contract unavailable"),
            Self::ContractMismatch => output.write_str("runtime control contract digest mismatch"),
            Self::StaleRevision { found } => write!(
                output,
                "control store platform revision {found} is stale; run explicit recovery"
            ),
            Self::Corrupt(reason) => write!(output, "corrupt control store: {reason}"),
            Self::Limit(name) => write!(output, "control store limit exceeded: {name}"),
            Self::InvalidKey => output.write_str("invalid canonical control key"),
            Self::SequenceExhausted => output.write_str("control sequence exhausted"),
        }
    }
}

impl std::error::Error for ControlStoreError {}

impl From<HostError> for ControlStoreError {
    fn from(value: HostError) -> Self {
        Self::Host(value)
    }
}
