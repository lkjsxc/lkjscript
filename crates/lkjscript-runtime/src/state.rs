use std::collections::{BTreeMap, VecDeque};
use std::num::{NonZeroU64, NonZeroUsize};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};

use lkjscript_core::ValidatedChunk;
use lkjscript_vm::ExecutionInputs;

use crate::cache::CodeCache;
use crate::{
    ApplicationId, ApplicationIncarnationId, ApplicationManifest, ApplicationStatus,
    CoordinatorIdentity, InvocationMetrics, Lifecycle, PackageContentId, ProcessCellState,
    ResourceAccounting, RuntimeError,
};

pub(crate) struct Inner {
    pub(crate) identity: CoordinatorIdentity,
    pub(crate) state: Mutex<State>,
    pub(crate) admission_changed: Condvar,
}

pub(crate) struct State {
    pub(crate) apps: BTreeMap<ApplicationId, AppRecord>,
    pub(crate) cache: CodeCache,
    pub(crate) global: GlobalAdmission,
    next_id: Option<NonZeroU64>,
}

impl State {
    pub(crate) fn new(max_cache_entries: NonZeroUsize, limits: crate::RuntimeLimits) -> Self {
        Self {
            apps: BTreeMap::new(),
            cache: CodeCache::new(max_cache_entries),
            global: GlobalAdmission::new(limits),
            next_id: Some(NonZeroU64::MIN),
        }
    }

    pub(crate) fn allocate(&mut self) -> Result<NonZeroU64, RuntimeError> {
        let id = self.next_id.ok_or(RuntimeError::IdentifierSpaceExhausted)?;
        self.next_id = id.get().checked_add(1).and_then(NonZeroU64::new);
        Ok(id)
    }
}

include!("system/global.rs");

pub(crate) struct AppRecord {
    pub(crate) manifest: ApplicationManifest,
    pub(crate) package: PackageContentId,
    pub(crate) chunk: Option<Arc<ValidatedChunk>>,
    pub(crate) process_spec: Option<IsolatedProcessSpec>,
    pub(crate) host: lkjscript_host::HostEnvironment,
    pub(crate) lifecycle: Lifecycle,
    pub(crate) incarnation_counter: u64,
    pub(crate) instance: Option<InstanceRuntime>,
}

impl AppRecord {
    pub(crate) fn incarnation(
        &self,
        coordinator: CoordinatorIdentity,
        app: ApplicationId,
    ) -> Option<ApplicationIncarnationId> {
        NonZeroU64::new(self.incarnation_counter)
            .map(|incarnation| ApplicationIncarnationId::new(coordinator, app, incarnation))
    }

    pub(crate) fn status(
        &self,
        coordinator: CoordinatorIdentity,
        app: ApplicationId,
    ) -> ApplicationStatus {
        let incarnation = self.incarnation(coordinator, app);
        let (cancelled, metrics, resources, logs) = match &self.instance {
            Some(runtime) => (
                runtime.cancelled,
                runtime.metrics,
                ResourceAccounting {
                    active_invocations: runtime.active,
                    total_invocations: runtime.total,
                },
                runtime.logs.iter().cloned().collect(),
            ),
            None => (
                false,
                InvocationMetrics::default(),
                ResourceAccounting {
                    active_invocations: 0,
                    total_invocations: 0,
                },
                Vec::new(),
            ),
        };
        let process_cell = match &self.manifest.cell {
            crate::ExecutionCellClass::TrustedInProcess => ProcessCellState::NotApplicable,
            crate::ExecutionCellClass::IsolatedProcess { .. } => match &self.instance {
                Some(runtime) if runtime.process_id.is_some() => ProcessCellState::Running {
                    process: runtime.process_id.unwrap_or(0),
                },
                _ if self.lifecycle == Lifecycle::Failed => ProcessCellState::Exited,
                _ if matches!(self.lifecycle, Lifecycle::Loading | Lifecycle::Starting) => {
                    ProcessCellState::Starting
                }
                _ => ProcessCellState::Stopped,
            },
        };
        ApplicationStatus {
            application: app,
            package: self.package,
            lifecycle: self.lifecycle,
            incarnation,
            cancelled,
            metrics,
            resources,
            logs,
            process_cell,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct IsolatedProcessSpec {
    pub(crate) worker: PathBuf,
    pub(crate) entry: PathBuf,
}

pub(crate) struct InstanceRuntime {
    pub(crate) id: ApplicationIncarnationId,
    pub(crate) inputs: ExecutionInputs,
    pub(crate) process: Option<Arc<Mutex<crate::execution::process::ProcessCell>>>,
    pub(crate) process_id: Option<u32>,
    pub(crate) cancelled: bool,
    pub(crate) logs: VecDeque<String>,
    pub(crate) metrics: InvocationMetrics,
    pub(crate) next_ticket: u64,
    pub(crate) serving_ticket: u64,
    pub(crate) active: usize,
    pub(crate) total: u64,
}

impl InstanceRuntime {
    pub(crate) fn new(
        id: ApplicationIncarnationId,
        inputs: ExecutionInputs,
        process: Option<Arc<Mutex<crate::execution::process::ProcessCell>>>,
        process_id: Option<u32>,
    ) -> Self {
        Self {
            id,
            inputs,
            process,
            process_id,
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
