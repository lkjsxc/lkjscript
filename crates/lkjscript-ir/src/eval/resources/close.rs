use lkjscript_core::{ProviderId, ResourceKind, ResourceTableError};

use super::*;

impl EvalResources {
    pub(crate) fn drop_owned(
        &mut self,
        resource: EvalResource,
        expected: ResourceKind,
    ) -> Result<(), String> {
        if expected == ResourceKind::InputStream
            || resource.kind != expected
            || resource.ownership != lkjscript_core::ResourceOwnership::Owned
        {
            return Err(format!(
                "implicit {} drop does not match an owned evaluator resource",
                expected.as_str()
            ));
        }
        let scope = self.table.scope();
        let closed = self.table.close_owned_with(
            resource.key,
            expected,
            resource.provider,
            scope,
            |observation, payload| payload.validate(&observation).map(|_| ()),
        );
        match closed {
            Ok(outcome) => {
                self.metrics.resources_closed = self.metrics.resources_closed.saturating_add(1);
                outcome
            }
            Err(error) => Err(error.to_string()),
        }
    }

    pub(crate) fn close_configured(&mut self, resource: EvalResource) -> Result<(), String> {
        if matches!(
            resource.kind,
            ResourceKind::SqliteConnection | ResourceKind::SqliteStatement
        ) {
            return Err("SQLite resources require close or finalize".into());
        }
        let kind = resource.kind;
        self.close_binding(resource, kind, provider_for_kind(kind))
    }

    #[cfg(test)]
    pub(super) fn close(&mut self, resource: EvalResource) -> Result<(), String> {
        if matches!(
            resource.kind,
            ResourceKind::SqliteConnection | ResourceKind::SqliteStatement
        ) {
            return Err("SQLite resources require close or finalize".into());
        }
        let kind = resource.kind;
        self.close_binding(resource, kind, provider_for_kind(kind))
    }

    pub(crate) fn close_sqlite_connection(&mut self, resource: EvalResource) -> Result<(), String> {
        self.close_binding(
            resource,
            ResourceKind::SqliteConnection,
            provider_for_kind(ResourceKind::SqliteConnection),
        )
    }

    pub(crate) fn finalize_statement(&mut self, resource: EvalResource) -> Result<(), String> {
        self.close_binding(
            resource,
            ResourceKind::SqliteStatement,
            provider_for_kind(ResourceKind::SqliteStatement),
        )
    }

    pub(super) fn close_binding(
        &mut self,
        resource: EvalResource,
        kind: ResourceKind,
        provider: ProviderId,
    ) -> Result<(), String> {
        let scope = self.scope();
        let fail_close = self.policy.fail_close == Some(kind);
        let result = self.table.close_owned_with(
            resource.key,
            kind,
            provider,
            scope,
            |observation, payload| {
                payload.validate(&observation)?;
                if fail_close {
                    Err("deterministic fake close failure".to_owned())
                } else {
                    Ok(())
                }
            },
        );
        match result {
            Ok(outcome) => {
                self.metrics.resources_closed = self.metrics.resources_closed.saturating_add(1);
                outcome
            }
            Err(error) => {
                self.record_access_error(&error);
                Err(error.to_string())
            }
        }
    }

    #[cfg(test)]
    pub(super) fn reject_borrowed_close(
        &mut self,
        resource: EvalResource,
    ) -> Result<(), ResourceTableError> {
        let scope = self.scope();
        let result = self
            .table
            .close_owned(resource.key, resource.kind, resource.provider, scope);
        match result {
            Ok(_) => Ok(()),
            Err(error) => {
                self.record_access_error(&error);
                Err(error)
            }
        }
    }

    pub(super) fn record_access_error(&mut self, error: &ResourceTableError) {
        if *error == ResourceTableError::StaleKey {
            self.metrics.stale_key_failures = self.metrics.stale_key_failures.saturating_add(1);
        }
    }
}
