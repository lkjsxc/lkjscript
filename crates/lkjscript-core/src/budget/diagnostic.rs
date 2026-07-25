use std::fmt;

use super::{BudgetAuthority, BudgetPath, ResourceCategory};
use crate::ResourceProfileIdentity;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetCause {
    Request,
    SemanticNode(u64),
    ArtifactSection(u32),
    ProtocolFrame(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetErrorKind {
    MissingAuthority,
    PathTooDeep,
    GrantExceedsParent,
    GrantBelowObserved,
    LimitExceeded,
    ReservationExceeded,
    ReservationIdExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetError {
    pub kind: BudgetErrorKind,
    pub profile: ResourceProfileIdentity,
    pub category: ResourceCategory,
    pub authority: Option<BudgetAuthority>,
    pub path: BudgetPath,
    pub cause: BudgetCause,
    pub limit: u64,
    pub reserved: u64,
    pub attempted: u64,
    pub observed: u64,
    pub allocated_before_rejection: bool,
}

impl fmt::Display for BudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let authority = self.authority.map_or("<missing>", BudgetAuthority::as_str);
        write!(
            formatter,
            "budget rejection: profile={}/{}:{}; category={}; unit={}; authority={}; path={}; kind={:?}; limit={}; reserved={}; attempted={}; observed={}; allocated-before-rejection={}",
            self.profile.schema,
            self.profile.version,
            self.profile.name.as_str(),
            self.category.as_str(),
            self.category.unit(),
            authority,
            self.path,
            self.kind,
            self.limit,
            self.reserved,
            self.attempted,
            self.observed,
            self.allocated_before_rejection,
        )
    }
}

impl std::error::Error for BudgetError {}
