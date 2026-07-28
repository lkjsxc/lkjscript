use lkjscript_core::{ProviderId, ResourceKind, ResourceOwnership, ScopeId};

use super::FakeOwner;

pub(super) struct FakeProviders {
    next_owner: Option<u64>,
}

impl FakeProviders {
    pub(super) const fn new() -> Self {
        Self {
            next_owner: Some(1),
        }
    }

    pub(super) fn borrowed(
        &mut self,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
    ) -> Result<FakeOwner, &'static str> {
        self.owner(kind, provider, scope, ResourceOwnership::Borrowed, None)
    }

    #[cfg(test)]
    pub(super) fn acquire_owned(
        &mut self,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
        parent: Option<u64>,
        succeeds: bool,
    ) -> Result<FakeOwner, &'static str> {
        if !succeeds {
            return Err("deterministic fake acquisition failure");
        }
        self.owner(kind, provider, scope, ResourceOwnership::Owned, parent)
    }

    fn owner(
        &mut self,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
        ownership: ResourceOwnership,
        parent: Option<u64>,
    ) -> Result<FakeOwner, &'static str> {
        let id = self
            .next_owner
            .ok_or("deterministic fake owner identity exhausted")?;
        self.next_owner = id.checked_add(1);
        Ok(FakeOwner {
            id,
            kind,
            provider,
            scope,
            ownership,
            parent,
        })
    }
}
