use lkjscript_core::ResourceTable;
#[cfg(test)]
use lkjscript_core::{ResourceOwnership, ResourceTableError, ScopeId};

use super::*;

pub(crate) struct EvalResources {
    pub(super) table: ResourceTable<FakeOwner>,
    pub(super) standard_input: Option<EvalResource>,
    pub(super) standard_output: Option<EvalResource>,
    #[cfg(test)]
    pub(super) providers: FakeProviders,
    pub(super) metrics: EvalResourceMetrics,
    pub(super) cleanup_failure_limits: lkjscript_core::CleanupFailureLimits,
}

impl EvalResources {
    #[cfg(test)]
    pub(super) const fn scope(&self) -> ScopeId {
        self.table.scope()
    }

    #[cfg(test)]
    pub(super) fn acquire_owned(
        &mut self,
        kind: lkjscript_core::ResourceKind,
        succeeds: bool,
    ) -> Result<EvalResource, String> {
        if ownership_for_kind(kind) != ResourceOwnership::Owned {
            return Err(format!("{} is evaluator-borrowed", kind.as_str()));
        }
        let provider = provider_for_kind(kind);
        let scope = self.scope();
        let reservation = self
            .table
            .reserve_owned(kind, provider)
            .map_err(|error| error.to_string())?;
        let payload = self
            .providers
            .acquire_owned(kind, provider, scope, None, succeeds);
        let key = commit_fake_acquisition(reservation, payload, &mut self.metrics)?;
        Ok(EvalResource::new(
            key,
            kind,
            provider,
            scope,
            ResourceOwnership::Owned,
        ))
    }

    #[cfg(test)]
    pub(super) fn prepare_statement(
        &mut self,
        connection: &EvalResource,
        succeeds: bool,
    ) -> Result<EvalResource, String> {
        use lkjscript_core::ResourceKind::{SqliteConnection, SqliteStatement};
        let provider = provider_for_kind(SqliteConnection);
        let scope = self.scope();
        let reservation = self
            .table
            .reserve_owned_child(&connection.key, SqliteConnection, SqliteStatement, provider)
            .map_err(|error| error.to_string())?;
        let parent = reservation
            .parent_payload()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "SQLite statement reservation has no parent".to_owned())?
            .id;
        let payload =
            self.providers
                .acquire_owned(SqliteStatement, provider, scope, Some(parent), succeeds);
        let key = commit_fake_acquisition(reservation, payload, &mut self.metrics)?;
        Ok(EvalResource::new(
            key,
            SqliteStatement,
            provider,
            scope,
            ResourceOwnership::Owned,
        ))
    }

    #[cfg(test)]
    pub(super) fn access_binding(
        &mut self,
        resource: &EvalResource,
        kind: lkjscript_core::ResourceKind,
        provider: lkjscript_core::ProviderId,
        ownership: ResourceOwnership,
    ) -> Result<(), ResourceTableError> {
        let scope = self.scope();
        let result = match ownership {
            ResourceOwnership::Owned => self.table.owned(&resource.key, kind, provider, scope),
            ResourceOwnership::Borrowed => {
                self.table.borrowed(&resource.key, kind, provider, scope)
            }
        };
        match result {
            Ok(payload) => payload
                .validate_binding(kind, provider, scope, ownership)
                .map_err(|_| ResourceTableError::StaleKey),
            Err(error) => {
                if error == ResourceTableError::StaleKey {
                    self.metrics.stale_key_failures =
                        self.metrics.stale_key_failures.saturating_add(1);
                }
                Err(error)
            }
        }
    }
}

#[cfg(test)]
fn commit_fake_acquisition(
    reservation: lkjscript_core::OwnedReservation<'_, FakeOwner>,
    payload: Result<FakeOwner, &'static str>,
    metrics: &mut EvalResourceMetrics,
) -> Result<lkjscript_core::ResourceKey, String> {
    let payload = match payload {
        Ok(payload) => payload,
        Err(error) => {
            metrics.failed_acquisitions = metrics.failed_acquisitions.saturating_add(1);
            return Err(error.to_owned());
        }
    };
    let key = reservation.commit(payload);
    metrics.resources_opened = metrics.resources_opened.saturating_add(1);
    if key.token_parts().generation().get() > 1 {
        metrics.slots_reused = metrics.slots_reused.saturating_add(1);
    }
    Ok(key)
}
