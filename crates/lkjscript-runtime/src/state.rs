use std::collections::{BTreeMap, VecDeque};
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::{Arc, Condvar, Mutex};

use lkjscript_core::ValidatedChunk;
use lkjscript_vm::ExecutionInputs;

use crate::cache::CodeCache;
use crate::{
    ApplicationGenerationId, ApplicationId, ApplicationInstanceId, ApplicationManifest,
    ApplicationStatus, InvocationMetrics, Lifecycle, NodeIdentity, PackageContentId,
    ProcessCellState, ResourceAccounting, RuntimeError,
};

pub(crate) struct Inner {
    pub(crate) identity: NodeIdentity,
    pub(crate) state: Mutex<State>,
    pub(crate) admission_changed: Condvar,
}

pub(crate) struct State {
    pub(crate) apps: BTreeMap<ApplicationId, AppRecord>,
    pub(crate) cache: CodeCache,
    next_id: Option<NonZeroU64>,
}

impl State {
    pub(crate) fn new(max_cache_entries: NonZeroUsize) -> Self {
        Self {
            apps: BTreeMap::new(),
            cache: CodeCache::new(max_cache_entries),
            next_id: Some(NonZeroU64::MIN),
        }
    }

    pub(crate) fn allocate(&mut self) -> Result<NonZeroU64, RuntimeError> {
        let id = self.next_id.ok_or(RuntimeError::IdentifierSpaceExhausted)?;
        self.next_id = id.get().checked_add(1).and_then(NonZeroU64::new);
        Ok(id)
    }
}

pub(crate) struct AppRecord {
    pub(crate) manifest: ApplicationManifest,
    pub(crate) package: PackageContentId,
    pub(crate) chunk: Option<Arc<ValidatedChunk>>,
    pub(crate) lifecycle: Lifecycle,
    pub(crate) generation_number: u64,
    pub(crate) instance: Option<InstanceRuntime>,
}

impl AppRecord {
    pub(crate) fn generation(&self, app: ApplicationId) -> Option<ApplicationGenerationId> {
        NonZeroU64::new(self.generation_number)
            .map(|generation| ApplicationGenerationId::new(app, generation))
    }

    pub(crate) fn status(&self, app: ApplicationId) -> ApplicationStatus {
        let generation = self.generation(app);
        let (instance, cancelled, metrics, resources, logs) = match &self.instance {
            Some(runtime) => (
                Some(runtime.id),
                runtime.cancelled,
                runtime.metrics,
                ResourceAccounting {
                    active_invocations: runtime.active,
                    total_invocations: runtime.total,
                },
                runtime.logs.iter().cloned().collect(),
            ),
            None => (
                None,
                false,
                InvocationMetrics::default(),
                ResourceAccounting {
                    active_invocations: 0,
                    total_invocations: 0,
                },
                Vec::new(),
            ),
        };
        ApplicationStatus {
            application: app,
            package: self.package,
            lifecycle: self.lifecycle,
            generation,
            instance,
            cancelled,
            metrics,
            resources,
            logs,
            process_cell: ProcessCellState::DeferredUnavailable,
        }
    }
}

pub(crate) struct InstanceRuntime {
    pub(crate) id: ApplicationInstanceId,
    pub(crate) inputs: ExecutionInputs,
    pub(crate) cancelled: bool,
    pub(crate) logs: VecDeque<String>,
    pub(crate) metrics: InvocationMetrics,
    pub(crate) next_ticket: u64,
    pub(crate) serving_ticket: u64,
    pub(crate) active: usize,
    pub(crate) total: u64,
}

impl InstanceRuntime {
    pub(crate) fn new(id: ApplicationInstanceId, inputs: ExecutionInputs) -> Self {
        Self {
            id,
            inputs,
            cancelled: false,
            logs: VecDeque::new(),
            metrics: InvocationMetrics::default(),
            next_ticket: 0,
            serving_ticket: 0,
            active: 0,
            total: 0,
        }
    }
}
