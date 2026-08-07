use super::resource_token::{encode_key, stdin_value};
use super::*;

impl Default for ResourceTable {
    fn default() -> Self {
        Self::new(None, CleanupRetentionPolicy::Unrestricted)
    }
}

impl ResourceTable {
    pub fn new(max_handles: Option<usize>, cleanup_retention: CleanupRetentionPolicy) -> Self {
        let (scope, scope_exhausted) = match next_scope() {
            Some(scope) => (scope, false),
            None => (exhausted_scope(), true),
        };
        let max_owned = max_handles
            .unwrap_or(TOKEN_SLOT_COUNT - 1)
            .min(TOKEN_SLOT_COUNT - 1);
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
            cleanup_retention,
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

    #[cfg(test)]
    pub fn inject_borrowed_cleanup_failure(&mut self) {
        let _removed = self.table.remove_borrowed(
            self.stdin_key.clone(),
            ResourceKind::InputStream,
            STDIO_PROVIDER,
            self.table.scope(),
        );
    }
}
