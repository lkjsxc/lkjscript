use std::num::NonZeroU64;

use lkjscript_contracts::ResourceKind;

use super::slot::SlotState;
use super::{
    ProviderId, ResourceKey, ResourceOwnership, ResourceTable, ResourceTableError, ScopeId,
};

/// Opaque-key coordinates carried by a bounded external token representation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ResourceTokenParts {
    slot: usize,
    generation: NonZeroU64,
}

impl ResourceTokenParts {
    pub const fn new(slot: usize, generation: NonZeroU64) -> Self {
        Self { slot, generation }
    }

    pub const fn slot(self) -> usize {
        self.slot
    }

    pub const fn generation(self) -> NonZeroU64 {
        self.generation
    }
}

impl ResourceKey {
    pub const fn token_parts(&self) -> ResourceTokenParts {
        ResourceTokenParts::new(self.slot, self.generation)
    }
}

impl<P> ResourceTable<P> {
    /// Resolve external token coordinates into a fully checked opaque key.
    pub fn resolve_token_parts(
        &self,
        parts: ResourceTokenParts,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
        ownership: ResourceOwnership,
    ) -> Result<ResourceKey, ResourceTableError> {
        if scope != self.scope {
            return Err(ResourceTableError::ScopeMismatch {
                expected: self.scope,
                actual: scope,
            });
        }
        let Some(slot) = self.slots.get(parts.slot) else {
            return Err(ResourceTableError::StaleKey);
        };
        if slot.generation != parts.generation {
            return Err(ResourceTableError::StaleKey);
        }
        let binding = match &slot.state {
            SlotState::OwnedOpen(open) | SlotState::BorrowedOpen(open) => open.binding,
            _ => return Err(ResourceTableError::StaleKey),
        };
        if binding.kind != kind {
            return Err(ResourceTableError::WrongKind {
                expected: kind,
                actual: binding.kind,
            });
        }
        if binding.provider != provider {
            return Err(ResourceTableError::ProviderMismatch {
                expected: provider,
                actual: binding.provider,
            });
        }
        if binding.scope != scope {
            return Err(ResourceTableError::ScopeMismatch {
                expected: scope,
                actual: binding.scope,
            });
        }
        if binding.ownership != ownership {
            return Err(ResourceTableError::OwnershipMismatch {
                expected: ownership,
                actual: binding.ownership,
            });
        }
        Ok(ResourceKey::new(
            parts.slot,
            parts.generation,
            binding.kind,
            binding.provider,
            binding.scope,
            binding.ownership,
        ))
    }
}
