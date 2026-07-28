use std::mem;

use lkjscript_contracts::ResourceKind;

use super::slot::{ClosingSlot, SlotState};
use super::{
    ProviderId, ResourceKey, ResourceObservation, ResourceOwnership, ResourceTable,
    ResourceTableError, ScopeId,
};

pub(super) struct PreparedClose<P> {
    pub(super) slot: usize,
    pub(super) payload: P,
    pub(super) observation: ResourceObservation,
}

impl<P> ResourceTable<P> {
    pub fn close_owned(
        &mut self,
        key: ResourceKey,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
    ) -> Result<P, ResourceTableError> {
        self.close_owned_with(key, kind, provider, scope, |_, payload| payload)
    }

    pub fn close_owned_with<R>(
        &mut self,
        key: ResourceKey,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
        close: impl FnOnce(ResourceObservation, P) -> R,
    ) -> Result<R, ResourceTableError> {
        let index = self.resolve(&key, kind, provider, scope, ResourceOwnership::Owned)?;
        let prepared = self.prepare_owned_close(index)?;
        self.remove_owned_order(index);
        let outcome = close(prepared.observation, prepared.payload);
        self.finish_close(prepared.slot);
        Ok(outcome)
    }

    pub fn remove_borrowed(
        &mut self,
        key: ResourceKey,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
    ) -> Result<P, ResourceTableError> {
        let index = self.resolve(&key, kind, provider, scope, ResourceOwnership::Borrowed)?;
        let children = match &self.slots[index].state {
            SlotState::BorrowedOpen(open) => open.live_children,
            state => {
                return Err(ResourceTableError::InvalidState {
                    state: state.into(),
                })
            }
        };
        if children != 0 {
            return Err(ResourceTableError::ParentHasLiveChildren { children });
        }
        let previous = mem::replace(&mut self.slots[index].state, SlotState::Closed);
        match previous {
            SlotState::BorrowedOpen(open) => {
                self.reusable.push(index);
                self.release_parent(open.binding.parent);
                Ok(open.payload)
            }
            state => {
                let actual = (&state).into();
                self.slots[index].state = state;
                Err(ResourceTableError::InvalidState { state: actual })
            }
        }
    }

    pub(super) fn prepare_owned_close(
        &mut self,
        index: usize,
    ) -> Result<PreparedClose<P>, ResourceTableError> {
        let children = match &self.slots[index].state {
            SlotState::OwnedOpen(open) => open.live_children,
            state => {
                return Err(ResourceTableError::InvalidState {
                    state: state.into(),
                })
            }
        };
        if children != 0 {
            return Err(ResourceTableError::ParentHasLiveChildren { children });
        }
        let previous = mem::replace(&mut self.slots[index].state, SlotState::Closed);
        match previous {
            SlotState::OwnedOpen(open) => {
                let observation = open.binding.observation(ResourceState::Closing, 0);
                self.slots[index].state = SlotState::Closing(ClosingSlot {
                    binding: open.binding,
                    sequence: open.sequence,
                    live_children: 0,
                });
                Ok(PreparedClose {
                    slot: index,
                    payload: open.payload,
                    observation,
                })
            }
            state => {
                let actual = (&state).into();
                self.slots[index].state = state;
                Err(ResourceTableError::InvalidState { state: actual })
            }
        }
    }

    pub(super) fn finish_close(&mut self, index: usize) {
        let previous = mem::replace(&mut self.slots[index].state, SlotState::Closed);
        if let SlotState::Closing(closing) = previous {
            self.reusable.push(index);
            self.release_parent(closing.binding.parent);
        } else {
            self.slots[index].state = previous;
        }
    }

    pub(super) fn remove_owned_order(&mut self, index: usize) {
        if let Some(position) = self.owned_order.iter().rposition(|slot| *slot == index) {
            self.owned_order.remove(position);
        }
    }
}

use super::ResourceState;
