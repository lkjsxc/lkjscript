use std::fmt;

use super::{
    BudgetAuthority, BudgetJournal, BudgetPath, BudgetPrefix, BudgetRejectedEvent,
    ResourceCategory, RESOURCE_CATEGORY_COUNT,
};
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
    JournalExhausted,
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
    prefix: BudgetPrefix,
}

impl BudgetError {
    pub(crate) fn new(
        profile: ResourceProfileIdentity,
        event: BudgetRejectedEvent,
        journal: &BudgetJournal,
        base: &[u64; RESOURCE_CATEGORY_COUNT],
    ) -> Self {
        Self {
            kind: event.kind,
            profile,
            category: event.category,
            authority: event.authority,
            path: event.path,
            cause: event.cause,
            limit: event.limit,
            reserved: event.reserved,
            attempted: event.attempted,
            observed: event.observed,
            allocated_before_rejection: event.allocated_before_rejection,
            prefix: journal.prefix(profile, base, Some(event)),
        }
    }

    pub const fn prefix(&self) -> &BudgetPrefix {
        &self.prefix
    }
}

impl fmt::Display for BudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let authority = self.authority.map_or("<missing>", BudgetAuthority::as_str);
        write!(
            formatter,
            concat!(
                "budget rejection: profile={}/{}:{}; category={}; unit={}; ",
                "authority={}; path={}; kind={:?}; cause={:?}; limit={}; reserved={}; ",
                "attempted={}; observed={}; allocated-before-rejection={}"
            ),
            self.profile.schema,
            self.profile.version,
            self.profile.name.as_str(),
            self.category.as_str(),
            self.category.unit(),
            authority,
            self.path,
            self.kind,
            self.cause,
            self.limit,
            self.reserved,
            self.attempted,
            self.observed,
            self.allocated_before_rejection,
        )
    }
}

impl std::error::Error for BudgetError {}
