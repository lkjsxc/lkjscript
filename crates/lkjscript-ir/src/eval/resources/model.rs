use std::fmt;

use lkjscript_core::{
    CapabilityKind, ProviderId, ResourceKey, ResourceKind, ResourceObservation, ResourceOwnership,
    ScopeId,
};

#[derive(Clone)]
pub struct EvalResource {
    pub(super) key: ResourceKey,
    pub(super) kind: ResourceKind,
    pub(super) provider: ProviderId,
    pub(super) scope: ScopeId,
    pub(super) ownership: ResourceOwnership,
}

impl EvalResource {
    pub(super) const fn new(
        key: ResourceKey,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
        ownership: ResourceOwnership,
    ) -> Self {
        Self {
            key,
            kind,
            provider,
            scope,
            ownership,
        }
    }
}

impl fmt::Debug for EvalResource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EvalResource")
            .field("kind", &self.kind)
            .field("provider", &self.provider)
            .field("scope", &self.scope)
            .field("ownership", &self.ownership)
            .field("binding", &"opaque")
            .finish()
    }
}

pub(super) struct FakeOwner {
    pub(super) id: u64,
    pub(super) kind: ResourceKind,
    pub(super) provider: ProviderId,
    pub(super) scope: ScopeId,
    pub(super) ownership: ResourceOwnership,
    pub(super) parent: Option<u64>,
}

impl FakeOwner {
    pub(super) fn validate(&self, observation: &ResourceObservation) -> Result<u64, String> {
        let valid = observation.kind() == Some(self.kind)
            && observation.provider() == Some(self.provider)
            && observation.scope() == Some(self.scope)
            && observation.ownership() == Some(self.ownership)
            && observation.has_parent() == self.parent.is_some();
        if valid {
            Ok(self.id)
        } else {
            Err(format!(
                "fake owner {} does not match its resource-table binding",
                self.id
            ))
        }
    }

    pub(super) fn validate_binding(
        &self,
        kind: ResourceKind,
        provider: ProviderId,
        scope: ScopeId,
        ownership: ResourceOwnership,
    ) -> Result<(), String> {
        if (self.kind, self.provider, self.scope, self.ownership)
            == (kind, provider, scope, ownership)
        {
            Ok(())
        } else {
            Err(format!("fake owner {} has an invalid binding", self.id))
        }
    }
}

pub(super) const fn provider_for_kind(kind: ResourceKind) -> ProviderId {
    let capability = match kind {
        ResourceKind::InputStream | ResourceKind::OutputStream => CapabilityKind::Stdio,
        ResourceKind::FileReader
        | ResourceKind::FileWriter
        | ResourceKind::FileAppender
        | ResourceKind::Directory => CapabilityKind::FileSystem,
        ResourceKind::TcpListener | ResourceKind::TcpStream => CapabilityKind::Network,
        ResourceKind::SqliteConnection | ResourceKind::SqliteStatement => CapabilityKind::Sqlite,
        ResourceKind::TerminalSession => CapabilityKind::Terminal,
    };
    ProviderId::for_capability(capability)
}

#[cfg(test)]
pub(super) const fn ownership_for_kind(kind: ResourceKind) -> ResourceOwnership {
    match kind {
        ResourceKind::InputStream | ResourceKind::OutputStream => ResourceOwnership::Borrowed,
        _ => ResourceOwnership::Owned,
    }
}
