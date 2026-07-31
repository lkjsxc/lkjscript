use super::*;

impl ResourceTeardown {
    pub const fn cleanup_failures(&self) -> &CleanupFailures {
        &self.cleanup_failures
    }

    pub const fn ordinary_obligations(&self) -> usize {
        self.ordinary_obligations
    }
}

#[cfg(test)]
impl ResourceTeardown {
    pub const fn emergency_obligations(&self) -> usize {
        self.emergency_obligations
    }

    pub const fn cleanup_attempts(&self) -> usize {
        self.cleanup_attempts
    }
}

impl ResourceTable {
    pub fn close(&mut self, handle: Value) -> Result<Value> {
        let (key, kind, provider) = self.resolve_owned_any(handle, "drop")?;
        if matches!(
            kind,
            ResourceKind::SqliteConnection | ResourceKind::SqliteStatement
        ) {
            return Err(Error::msg(
                "drop: SQLite handles require their SQLite close operation",
            ));
        }
        self.close_owned_key(key, kind, provider, "drop")
    }

    pub(super) fn close_owned_key(
        &mut self,
        key: ResourceKey,
        kind: ResourceKind,
        provider: ProviderId,
        operation: &str,
    ) -> Result<Value> {
        let scope = self.table.scope();
        let closed = self
            .table
            .close_owned_with(key, kind, provider, scope, |_, payload| {
                close_payload(kind, payload, operation)
            });
        match closed {
            Ok(outcome) => {
                self.record_closed(1);
                outcome
            }
            Err(error) => Err(self.access_error(operation, error)),
        }
    }

    pub fn teardown(&mut self) -> ResourceTeardown {
        let ordinary_obligations = match self.table.assert_zero_ordinary_obligations() {
            Ok(()) => 0,
            Err(ResourceTableError::OutstandingOrdinaryObligations { count }) => count,
            Err(_) => self.table.stats().ordinary_obligations(),
        };
        let emergency_obligations = self.table.emergency_obligations().count();
        let mut cleanup_failures = CleanupFailures::new(self.cleanup_failure_limits);
        let cleanup_attempts = match self.table.cleanup_owned_reverse(|_, payload| drop(payload)) {
            Ok(report) => report.count(),
            Err(error) => {
                cleanup_failures.push(
                    lkjscript_core::CleanupPhase::Emergency,
                    lkjscript_core::CleanupSubject::ResourceTable,
                    error.to_string(),
                );
                0
            }
        };
        self.record_closed(cleanup_attempts);
        if let Err(error) = self.table.remove_borrowed(
            self.stdin_key.clone(),
            ResourceKind::InputStream,
            STDIO_PROVIDER,
            self.table.scope(),
        ) {
            cleanup_failures.push(
                lkjscript_core::CleanupPhase::RuntimeTeardown,
                lkjscript_core::CleanupSubject::BorrowedResource(ResourceKind::InputStream),
                format!("borrowed standard input removal failed: {error}"),
            );
        }
        self.update_metrics(|metrics| {
            metrics.ordinary_obligations = ordinary_obligations;
            metrics.emergency_obligations = emergency_obligations;
            metrics.cleanup_attempts = cleanup_attempts;
        });
        ResourceTeardown {
            ordinary_obligations,
            emergency_obligations,
            cleanup_attempts,
            cleanup_failures,
        }
    }
}

fn close_payload(kind: ResourceKind, payload: OwnedResource, operation: &str) -> Result<Value> {
    let valid = matches!(
        (kind, &payload),
        (
            ResourceKind::FileReader | ResourceKind::FileWriter | ResourceKind::FileAppender,
            OwnedResource::File(_),
        ) | (ResourceKind::Directory, OwnedResource::Directory(_))
            | (
                ResourceKind::TcpListener | ResourceKind::TcpStream,
                OwnedResource::Socket(_),
            )
            | (
                ResourceKind::SqliteConnection,
                OwnedResource::SqliteConnection(_),
            )
            | (
                ResourceKind::SqliteStatement,
                OwnedResource::SqliteStatement(_),
            )
    );
    drop(payload);
    if valid {
        Ok(Value::UNIT)
    } else {
        Err(Error::msg(format!(
            "{operation}: resource payload does not match {}",
            kind.as_str()
        )))
    }
}
