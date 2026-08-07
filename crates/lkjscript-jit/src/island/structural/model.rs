use super::*;

pub(in crate::island) struct JitStructuralRuntime {
    pub(super) runtime: StructuralValueRuntime,
    pub(super) calls: u64,
    pub(super) last_resource: Option<ResourceLimitKind>,
    pub(super) owners: std::collections::BTreeMap<u64, NativeOwnerRecord>,
    pub(super) last_trap: Option<String>,
}

impl JitStructuralRuntime {
    pub(in crate::island) fn new(config: &ExecutionPolicy) -> Result<Self, EngineError> {
        let mut limits = StructuralValueRuntimeLimits::default();
        if let Some(policy) = config.limited_policy() {
            let handles = u32::try_from(policy.max_handles).unwrap_or(u32::MAX).max(1);
            limits.max_objects = limits.max_objects.min(handles);
            limits.max_destinations = limits.max_destinations.min(handles);
            limits.max_views = limits.max_views.min(handles);
            limits.max_payload_bytes = limits
                .max_payload_bytes
                .min(u64::try_from(policy.max_heap_bytes).unwrap_or(u64::MAX))
                .max(1);
        }
        let runtime = StructuralValueRuntime::new(limits).map_err(|error| {
            EngineError::new(
                FailureCode::InvocationFailure,
                None,
                format!("native structural runtime configuration: {error}"),
            )
        })?;
        Ok(Self {
            runtime,
            calls: 0,
            last_resource: None,
            owners: std::collections::BTreeMap::new(),
            last_trap: None,
        })
    }

    pub(in crate::island) fn export(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<SemanticValue, NativeServiceError> {
        self.note_call();
        let expected = core_type(owner.structural_type())?;
        let key = owner_key(owner)?;
        let value = self
            .runtime
            .export_semantic(key, expected)
            .map_err(|error| self.map_error(error))?;
        self.owners.remove(&key.get());
        Ok(value)
    }

    pub(in crate::island) fn finish(
        mut self,
    ) -> (NativeStructuralStats, Option<ResourceLimitKind>) {
        self.cleanup_owners();
        let empty = self.owners.is_empty() && self.runtime.verify_empty().is_ok();
        let roots = self.runtime.root_stats();
        let metrics = self.runtime.metrics();
        let stats = NativeStructuralStats {
            calls: self.calls,
            roots_published: roots.roots_published,
            roots_moved: roots.roots_moved,
            roots_dropped: roots.roots_dropped,
            roots_released: roots.roots_released,
            loans_started: roots.loans_started,
            loans_ended: roots.loans_ended,
            destinations_created: metrics.destinations_created,
            destinations_completed: metrics.destinations_completed,
            destinations_aborted: metrics.destinations_aborted,
            views_created: metrics.views_created,
            views_ended: metrics.views_ended,
            event_records: self.runtime.events().len() as u64,
            events_overwritten: metrics.events_overwritten,
            releases: metrics.releases,
            release_work: metrics.release_work,
            sealed_publications: metrics.sealed_publications,
            zero_copy_adoptions: metrics.zero_copy_adoptions,
            copied_publication_bytes: metrics.copied_publication_bytes,
            sealed_acquisitions: metrics.sealed_acquisitions,
            sealed_releases: metrics.sealed_releases,
            sealed_release_work: metrics.sealed_release_work,
            sealed_nodes_reclaimed: metrics.sealed_nodes_reclaimed,
            live_objects: metrics.live_objects,
            live_sealed_domains: metrics.live_sealed_domains,
            live_sealed_owners: metrics.live_sealed_owners,
            live_roots: roots.live_roots,
            live_loans: roots.live_loans,
            live_views: metrics.live_views,
            live_destinations: metrics.live_destinations,
            release_backlog: metrics.release_backlog,
            empty_completions: u64::from(empty),
            teardown_failures: u64::from(!empty),
        };
        (stats, self.last_resource)
    }

    fn cleanup_owners(&mut self) {
        let count = self.owners.len();
        let mut owners = Vec::new();
        if owners.try_reserve_exact(count).is_err() {
            self.last_resource = Some(ResourceLimitKind::Allocations);
            self.last_trap = Some("copy structural teardown allocation failed".into());
            return;
        }
        owners.extend(
            self.owners
                .iter()
                .map(|(key, record)| (*key, record.value_type)),
        );
        for (key, value_type) in owners {
            let owner = NativeStructuralOwner::new(value_type, key);
            if self.drop_owner(owner).is_err() {
                self.owners.remove(&key);
            }
        }
    }

    pub(in crate::island) fn take_last_trap(&mut self) -> Option<String> {
        self.last_trap.take()
    }

    pub(in crate::island) fn record_trap(&mut self, message: impl Into<String>) {
        self.last_trap = Some(message.into());
    }

    pub(super) fn note_call(&mut self) {
        self.calls = self.calls.saturating_add(1);
    }

    pub(super) fn map_error(&mut self, error: StructuralValueError) -> NativeServiceError {
        use StructuralValueError as Error;
        match error {
            Error::AllocationFailed
            | Error::LimitExceeded(_)
            | Error::Domain(
                StructuralError::AllocationFailed | StructuralError::LimitExceeded(_),
            )
            | Error::RootTable(
                StructuralRootTableError::AllocationFailed
                | StructuralRootTableError::LimitExceeded(_),
            ) => {
                self.last_resource = Some(ResourceLimitKind::Handles);
                NativeServiceError::ResourceLimitExceeded
            }
            Error::ArithmeticOverflow
            | Error::InvalidLimits
            | Error::InvariantViolation
            | Error::Domain(
                StructuralError::ArithmeticOverflow | StructuralError::RuntimeIdentityExhausted,
            )
            | Error::RootTable(
                StructuralRootTableError::ArithmeticOverflow
                | StructuralRootTableError::InvalidLimits
                | StructuralRootTableError::InvariantViolation,
            ) => NativeServiceError::HostFailure,
            Error::Domain(_)
            | Error::RootTable(_)
            | Error::StaleObject
            | Error::StaleDestination
            | Error::StaleView
            | Error::WrongLayout
            | Error::WrongSemanticType
            | Error::WrongPayloadKind
            | Error::InvalidUtf8
            | Error::InvalidPath
            | Error::InvalidRange
            | Error::InvalidFieldPath
            | Error::MixedValue
            | Error::FieldAlreadyInitialized
            | Error::FieldOutOfRange
            | Error::IncompleteDestination
            | Error::WrongDestinationKind
            | Error::WrongOwnership
            | Error::OwnerOverflow
            | Error::LiveDestination
            | Error::LiveView
            | Error::LiveObject
            | Error::ReleaseBacklog => NativeServiceError::Trap,
        }
    }
}
