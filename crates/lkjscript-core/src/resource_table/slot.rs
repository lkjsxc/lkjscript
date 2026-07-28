use std::num::NonZeroU64;

use lkjscript_contracts::ResourceKind;

use super::{ProviderId, ResourceObservation, ResourceOwnership, ResourceState, ScopeId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ParentRef {
    pub(super) slot: usize,
    pub(super) generation: NonZeroU64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Binding {
    pub(super) kind: ResourceKind,
    pub(super) provider: ProviderId,
    pub(super) scope: ScopeId,
    pub(super) ownership: ResourceOwnership,
    pub(super) parent: Option<ParentRef>,
}

impl Binding {
    pub(super) const fn observation(
        self,
        state: ResourceState,
        live_children: usize,
    ) -> ResourceObservation {
        ResourceObservation {
            state,
            kind: Some(self.kind),
            provider: Some(self.provider),
            scope: Some(self.scope),
            ownership: Some(self.ownership),
            has_parent: self.parent.is_some(),
            live_children,
        }
    }
}

pub(super) struct ReservedSlot {
    pub(super) binding: Binding,
}

pub(super) struct OpenSlot<P> {
    pub(super) binding: Binding,
    pub(super) payload: P,
    pub(super) sequence: u64,
    pub(super) live_children: usize,
}

pub(super) struct ClosingSlot {
    pub(super) binding: Binding,
    pub(super) sequence: u64,
    pub(super) live_children: usize,
}

pub(super) enum SlotState<P> {
    Vacant,
    Reserved(ReservedSlot),
    OwnedOpen(OpenSlot<P>),
    BorrowedOpen(OpenSlot<P>),
    Closing(ClosingSlot),
    Closed,
    Retired,
}

pub(super) struct Slot<P> {
    pub(super) generation: NonZeroU64,
    pub(super) state: SlotState<P>,
}

impl<P> Slot<P> {
    pub(super) const fn new(generation: NonZeroU64) -> Self {
        Self {
            generation,
            state: SlotState::Vacant,
        }
    }

    pub(super) fn observation(&self) -> ResourceObservation {
        match &self.state {
            SlotState::Vacant => ResourceObservation::inactive(ResourceState::Vacant),
            SlotState::Reserved(slot) => slot.binding.observation(ResourceState::Reserved, 0),
            SlotState::OwnedOpen(slot) => slot
                .binding
                .observation(ResourceState::OwnedOpen, slot.live_children),
            SlotState::BorrowedOpen(slot) => slot
                .binding
                .observation(ResourceState::BorrowedOpen, slot.live_children),
            SlotState::Closing(slot) => slot
                .binding
                .observation(ResourceState::Closing, slot.live_children),
            SlotState::Closed => ResourceObservation::inactive(ResourceState::Closed),
            SlotState::Retired => ResourceObservation::inactive(ResourceState::Retired),
        }
    }

    pub(super) const fn acquisition_sequence(&self) -> Option<u64> {
        match &self.state {
            SlotState::OwnedOpen(slot) => Some(slot.sequence),
            SlotState::Closing(slot) => Some(slot.sequence),
            _ => None,
        }
    }
}
