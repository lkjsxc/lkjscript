use std::fmt;
use std::num::NonZeroU64;

use lkjscript_contracts::{CapabilityKind, ResourceKind};

use super::ResourceOwnership;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProviderId(NonZeroU64);

impl ProviderId {
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn for_capability(kind: CapabilityKind) -> Self {
        let value = (kind as u64) + 1;
        match NonZeroU64::new(value) {
            Some(value) => Self(value),
            None => Self(NonZeroU64::MIN),
        }
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScopeId(NonZeroU64);

impl ScopeId {
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ResourceKey {
    pub(super) slot: usize,
    pub(super) generation: NonZeroU64,
    pub(super) kind: ResourceKind,
    pub(super) provider: ProviderId,
    pub(super) scope: ScopeId,
    pub(super) ownership: ResourceOwnership,
}

impl ResourceKey {
    pub(super) const fn new(
        slot: usize,
        generation: NonZeroU64,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
        ownership: ResourceOwnership,
    ) -> Self {
        Self {
            slot,
            generation,
            kind,
            provider,
            scope,
            ownership,
        }
    }
}

impl fmt::Debug for ResourceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ResourceKey(opaque)")
    }
}
