use super::resource_token::{encode_key, stdin_value};
use super::*;

impl Default for ResourceTable {
    fn default() -> Self {
        Self::new(4_096)
    }
}

impl ResourceTable {
    pub fn new(max_handles: usize) -> Self {
        let (scope, scope_exhausted) = match next_scope() {
            Some(scope) => (scope, false),
            None => (exhausted_scope(), true),
        };
        let max_owned = max_handles.min(TOKEN_SLOT_COUNT - 1);
        let limits = ResourceTableLimits::new(
            max_owned + 1,
            max_owned + 1,
            max_owned,
            1,
            max_owned,
            TOKEN_MAX_GENERATION,
        )
        .unwrap_or_else(|_| std::process::abort());
        let mut table = CoreResourceTable::new(scope, limits);
        let stdin_key = table
            .reserve_borrowed(ResourceKind::InputStream, STDIO_PROVIDER)
            .unwrap_or_else(|_| std::process::abort())
            .commit(OwnedResource::StandardInput);
        if encode_key(&stdin_key).is_err() {
            std::process::abort();
        }
        Self {
            table,
            stdin_key,
            metrics: Cell::new(ResourceMetrics::default()),
            limit_exceeded: false,
            scope_exhausted,
        }
    }

    pub fn allocated_handle_slots(&self) -> usize {
        self.table.stats().allocated_slots().saturating_sub(1)
    }

    pub const fn limit_exceeded(&self) -> bool {
        self.limit_exceeded
    }

    pub fn stdin_handle() -> Value {
        stdin_value()
    }

    #[cfg(test)]
    pub fn metrics(&self) -> ResourceMetrics {
        self.metrics.get()
    }

    #[cfg(test)]
    pub const fn scope_id(&self) -> ScopeId {
        self.table.scope()
    }
}
