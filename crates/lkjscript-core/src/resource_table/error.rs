use std::error::Error;
use std::fmt;

use lkjscript_contracts::ResourceKind;

use super::{ProviderId, ResourceOwnership, ResourceState, ScopeId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceTableLimit {
    Slots,
    Reservations,
    Owned,
    Borrowed,
    ChildrenPerParent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceTableError {
    LimitExceeded {
        limit: ResourceTableLimit,
        configured: usize,
        observed: usize,
    },
    GenerationExhausted {
        retired_slots: usize,
    },
    AcquisitionSequenceExhausted,
    StaleKey,
    WrongKind {
        expected: ResourceKind,
        actual: ResourceKind,
    },
    ProviderMismatch {
        expected: ProviderId,
        actual: ProviderId,
    },
    ScopeMismatch {
        expected: ScopeId,
        actual: ScopeId,
    },
    OwnershipMismatch {
        expected: ResourceOwnership,
        actual: ResourceOwnership,
    },
    InvalidState {
        state: ResourceState,
    },
    ParentHasLiveChildren {
        children: usize,
    },
    OutstandingOrdinaryObligations {
        count: usize,
    },
}

impl fmt::Display for ResourceTableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitExceeded {
                limit,
                configured,
                observed,
            } => write!(
                formatter,
                "resource table {limit:?} limit {configured} exceeded at {observed}"
            ),
            Self::GenerationExhausted { retired_slots } => {
                write!(
                    formatter,
                    "resource generations exhausted; {retired_slots} slots retired"
                )
            }
            Self::AcquisitionSequenceExhausted => {
                formatter.write_str("resource acquisition sequence exhausted")
            }
            Self::StaleKey => formatter.write_str("stale resource key"),
            Self::WrongKind { expected, actual } => write!(
                formatter,
                "resource kind mismatch: expected {}, got {}",
                expected.as_str(),
                actual.as_str()
            ),
            Self::ProviderMismatch { expected, actual } => write!(
                formatter,
                "resource provider mismatch: expected {}, got {}",
                expected.get(),
                actual.get()
            ),
            Self::ScopeMismatch { expected, actual } => write!(
                formatter,
                "resource scope mismatch: expected {}, got {}",
                expected.get(),
                actual.get()
            ),
            Self::OwnershipMismatch { expected, actual } => write!(
                formatter,
                "resource ownership mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::InvalidState { state } => {
                write!(formatter, "resource is not open: state is {state:?}")
            }
            Self::ParentHasLiveChildren { children } => {
                write!(formatter, "resource has {children} live children")
            }
            Self::OutstandingOrdinaryObligations { count } => {
                write!(formatter, "resource table has {count} ordinary obligations")
            }
        }
    }
}

impl Error for ResourceTableError {}
