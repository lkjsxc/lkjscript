use lkjscript_contracts::ResourceKind;

use super::slot::{Binding, SlotState};
use super::{
    ProviderId, ResourceKey, ResourceOwnership, ResourceTable, ResourceTableError, ScopeId,
};

impl<P> ResourceTable<P> {
    pub fn owned(
        &self,
        key: &ResourceKey,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
    ) -> Result<&P, ResourceTableError> {
        let index = self.resolve(key, kind, provider, scope, ResourceOwnership::Owned)?;
        match &self.slots[index].state {
            SlotState::OwnedOpen(open) => Ok(&open.payload),
            state => Err(ResourceTableError::InvalidState {
                state: state.into(),
            }),
        }
    }

    pub fn owned_mut(
        &mut self,
        key: &ResourceKey,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
    ) -> Result<&mut P, ResourceTableError> {
        let index = self.resolve(key, kind, provider, scope, ResourceOwnership::Owned)?;
        match &mut self.slots[index].state {
            SlotState::OwnedOpen(open) => Ok(&mut open.payload),
            state => Err(ResourceTableError::InvalidState {
                state: (&*state).into(),
            }),
        }
    }

    pub fn borrowed(
        &self,
        key: &ResourceKey,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
    ) -> Result<&P, ResourceTableError> {
        let index = self.resolve(key, kind, provider, scope, ResourceOwnership::Borrowed)?;
        match &self.slots[index].state {
            SlotState::BorrowedOpen(open) => Ok(&open.payload),
            state => Err(ResourceTableError::InvalidState {
                state: state.into(),
            }),
        }
    }

    pub fn borrowed_mut(
        &mut self,
        key: &ResourceKey,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
    ) -> Result<&mut P, ResourceTableError> {
        let index = self.resolve(key, kind, provider, scope, ResourceOwnership::Borrowed)?;
        match &mut self.slots[index].state {
            SlotState::BorrowedOpen(open) => Ok(&mut open.payload),
            state => Err(ResourceTableError::InvalidState {
                state: (&*state).into(),
            }),
        }
    }

    pub(super) fn resolve(
        &self,
        key: &ResourceKey,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
        ownership: ResourceOwnership,
    ) -> Result<usize, ResourceTableError> {
        self.validate_key_binding(key, kind, provider, scope, ownership)?;
        let Some(slot) = self.slots.get(key.slot) else {
            return Err(ResourceTableError::StaleKey);
        };
        if slot.generation != key.generation {
            return Err(ResourceTableError::StaleKey);
        }
        let binding = match &slot.state {
            SlotState::OwnedOpen(open) | SlotState::BorrowedOpen(open) => open.binding,
            _ => return Err(ResourceTableError::StaleKey),
        };
        if !Self::binding_matches_key(binding, key) {
            return Err(ResourceTableError::StaleKey);
        }
        Ok(key.slot)
    }

    pub(super) fn validate_key_binding(
        &self,
        key: &ResourceKey,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
        ownership: ResourceOwnership,
    ) -> Result<(), ResourceTableError> {
        if key.kind != kind {
            return Err(ResourceTableError::WrongKind {
                expected: kind,
                actual: key.kind,
            });
        }
        if key.provider != provider {
            return Err(ResourceTableError::ProviderMismatch {
                expected: provider,
                actual: key.provider,
            });
        }
        if key.scope != scope {
            return Err(ResourceTableError::ScopeMismatch {
                expected: scope,
                actual: key.scope,
            });
        }
        if scope != self.scope {
            return Err(ResourceTableError::ScopeMismatch {
                expected: self.scope,
                actual: scope,
            });
        }
        if key.ownership != ownership {
            return Err(ResourceTableError::OwnershipMismatch {
                expected: ownership,
                actual: key.ownership,
            });
        }
        Ok(())
    }

    pub(super) fn binding_matches_key(binding: Binding, key: &ResourceKey) -> bool {
        binding.kind == key.kind
            && binding.provider == key.provider
            && binding.scope == key.scope
            && binding.ownership == key.ownership
    }
}

impl<P> From<&SlotState<P>> for super::ResourceState {
    fn from(state: &SlotState<P>) -> Self {
        match state {
            SlotState::Vacant => Self::Vacant,
            SlotState::Reserved(_) => Self::Reserved,
            SlotState::OwnedOpen(_) => Self::OwnedOpen,
            SlotState::BorrowedOpen(_) => Self::BorrowedOpen,
            SlotState::Closing(_) => Self::Closing,
            SlotState::Closed => Self::Closed,
            SlotState::Retired => Self::Retired,
        }
    }
}
