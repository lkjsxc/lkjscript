use super::resource_token::{decode_parts, encode_key, provider_for_kind, reservation_exhausted};
use super::*;

impl ResourceTable {
    pub(crate) fn is_borrowed_handle(&self, value: Value) -> bool {
        encode_key(&self.stdin_key).is_ok_and(|borrowed| borrowed == value)
    }

    pub(super) fn acquire_owned(
        &mut self,
        kind: ResourceKind,
        provider: ProviderId,
        operation: &str,
        acquire: impl FnOnce() -> Result<OwnedResource>,
    ) -> Result<Value> {
        let reservation = self.reserve_owned(kind, provider, operation)?;
        let payload = acquire()?;
        let key = reservation.commit(payload);
        self.publish_owned(key, kind, provider)
    }

    pub(super) fn reserve_owned(
        &mut self,
        kind: ResourceKind,
        provider: ProviderId,
        operation: &str,
    ) -> Result<OwnedReservation<'_, OwnedResource>> {
        if self.scope_exhausted {
            self.limit_exceeded = true;
            return Err(Error::msg(format!("{operation}: resource scope exhausted")));
        }
        match self.table.reserve_owned(kind, provider) {
            Ok(reservation) => Ok(reservation),
            Err(error) => {
                if reservation_exhausted(&error) {
                    self.limit_exceeded = true;
                }
                Err(Error::msg(format!("{operation}: {error}")))
            }
        }
    }

    pub(super) fn reserve_owned_child(
        &mut self,
        parent: &ResourceKey,
        parent_kind: ResourceKind,
        child_kind: ResourceKind,
        provider: ProviderId,
        operation: &str,
    ) -> Result<OwnedReservation<'_, OwnedResource>> {
        if self.scope_exhausted {
            self.limit_exceeded = true;
            return Err(Error::msg(format!("{operation}: resource scope exhausted")));
        }
        match self
            .table
            .reserve_owned_child(parent, parent_kind, child_kind, provider)
        {
            Ok(reservation) => Ok(reservation),
            Err(error) => {
                if reservation_exhausted(&error) {
                    self.limit_exceeded = true;
                }
                Err(Error::msg(format!("{operation}: {error}")))
            }
        }
    }

    pub(super) fn publish_owned(
        &mut self,
        key: ResourceKey,
        kind: ResourceKind,
        provider: ProviderId,
    ) -> Result<Value> {
        match encode_key(&key) {
            Ok(value) => {
                self.update_metrics(|metrics| {
                    metrics.resources_opened = metrics.resources_opened.saturating_add(1);
                    if key.token_parts().generation().get() > 1 {
                        metrics.slots_reused = metrics.slots_reused.saturating_add(1);
                    }
                });
                Ok(value)
            }
            Err(error) => {
                self.limit_exceeded = true;
                let scope = self.table.scope();
                let _ = self
                    .table
                    .close_owned_with(key, kind, provider, scope, |_, payload| drop(payload));
                self.record_closed(1);
                Err(error)
            }
        }
    }

    pub(super) fn resolve_exact(
        &self,
        handle: Value,
        kind: ResourceKind,
        provider: ProviderId,
        ownership: ResourceOwnership,
        operation: &str,
    ) -> Result<ResourceKey> {
        let parts = decode_parts(handle, operation)?;
        self.table
            .resolve_token_parts(parts, kind, provider, self.table.scope(), ownership)
            .map_err(|error| self.access_error(operation, error))
    }

    pub(crate) fn owned_kind(&self, handle: Value, operation: &str) -> Result<ResourceKind> {
        self.resolve_owned_any(handle, operation)
            .map(|(_, kind, _)| kind)
    }

    pub(super) fn resolve_owned_any(
        &self,
        handle: Value,
        operation: &str,
    ) -> Result<(ResourceKey, ResourceKind, ProviderId)> {
        let parts = decode_parts(handle, operation)?;
        for kind in ResourceKind::ALL {
            let provider = provider_for_kind(kind);
            match self.table.resolve_token_parts(
                parts,
                kind,
                provider,
                self.table.scope(),
                ResourceOwnership::Owned,
            ) {
                Ok(key) => return Ok((key, kind, provider)),
                Err(ResourceTableError::WrongKind { .. }) => {}
                Err(error) => return Err(self.access_error(operation, error)),
            }
        }
        Err(Error::msg(format!("{operation}: unknown resource kind")))
    }

    pub(super) fn access_error(&self, operation: &str, error: ResourceTableError) -> Error {
        if error == ResourceTableError::StaleKey {
            self.update_metrics(|metrics| {
                metrics.stale_key_failures = metrics.stale_key_failures.saturating_add(1);
            });
        }
        Error::msg(format!("{operation}: {error}"))
    }

    pub(super) fn record_closed(&self, count: usize) {
        self.update_metrics(|metrics| {
            metrics.resources_closed = metrics
                .resources_closed
                .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
        });
    }

    pub(super) fn update_metrics(&self, update: impl FnOnce(&mut ResourceMetrics)) {
        let mut metrics = self.metrics.get();
        update(&mut metrics);
        self.metrics.set(metrics);
    }
}
