use super::resource_token::encode_key;
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
        let max_slots = max_handles.and_then(|maximum| maximum.checked_add(1));
        let limits = ResourceTableLimits::optional(
            max_slots,
            max_slots,
            max_handles,
            Some(1),
            max_handles,
            None,
        )
        .unwrap_or_else(|_| std::process::abort());
        let mut core = CoreResourceTable::new(scope, limits);
        let stdin_key = core
            .reserve_borrowed(ResourceKind::InputStream, STDIO_PROVIDER)
            .unwrap_or_else(|_| std::process::abort())
            .commit(OwnedResource::StandardInput);
        let mut resources = Self {
            table: core,
            stdin_key,
            tokens: HashMap::new(),
            token_by_identity: HashMap::new(),
            next_token: NonZeroU64::new(1),
            metrics: Cell::new(ResourceMetrics::default()),
            limit_exceeded: false,
            scope_exhausted,
            cleanup_retention,
        };
        let stdin_key = resources.stdin_key.clone();
        let stdin =
            encode_key(&mut resources, &stdin_key).unwrap_or_else(|_| std::process::abort());
        if stdin != Self::stdin_handle() {
            std::process::abort();
        }
        resources
    }

    pub fn allocated_handle_slots(&self) -> usize {
        self.table.stats().allocated_slots().saturating_sub(1)
    }

    pub const fn limit_exceeded(&self) -> bool {
        self.limit_exceeded
    }

    pub fn stdin_handle() -> Value {
        Value::from_resource(1)
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
    pub fn decode_handle_for_test(&self, handle: Value) -> Result<ResourceTokenParts> {
        super::resource_token::decode_parts(self, handle, "test")
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
