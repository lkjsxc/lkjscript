use lkjscript_contracts::ResourceKind;

use super::{ProviderId, ScopeId};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResourceOwnership {
    Owned,
    Borrowed,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResourceState {
    Vacant,
    Reserved,
    OwnedOpen,
    BorrowedOpen,
    Closing,
    Closed,
    Retired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceObservation {
    pub(super) state: ResourceState,
    pub(super) kind: Option<ResourceKind>,
    pub(super) provider: Option<ProviderId>,
    pub(super) scope: Option<ScopeId>,
    pub(super) ownership: Option<ResourceOwnership>,
    pub(super) has_parent: bool,
    pub(super) live_children: usize,
}

impl ResourceObservation {
    pub const fn state(&self) -> ResourceState {
        self.state
    }

    pub const fn kind(&self) -> Option<ResourceKind> {
        self.kind
    }

    pub const fn provider(&self) -> Option<ProviderId> {
        self.provider
    }

    pub const fn scope(&self) -> Option<ScopeId> {
        self.scope
    }

    pub const fn ownership(&self) -> Option<ResourceOwnership> {
        self.ownership
    }

    pub const fn has_parent(&self) -> bool {
        self.has_parent
    }

    pub const fn live_children(&self) -> usize {
        self.live_children
    }

    pub(super) const fn inactive(state: ResourceState) -> Self {
        Self {
            state,
            kind: None,
            provider: None,
            scope: None,
            ownership: None,
            has_parent: false,
            live_children: 0,
        }
    }
}
