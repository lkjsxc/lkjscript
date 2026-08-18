//! Topology-neutral operational composition for exact applications and durable instances.

use crate::application::{
    self, APPLICATION_CONTRACT_VERSION, ApplicationInspection, ApplicationInvocation,
    ApplicationLoadObservation, ApplicationRunReceipt, ApplicationTestReport, HostInterface,
    HostInterfaceId, MAXIMUM_APPLICATION_ARTIFACT_BYTES,
};
use crate::error::{ErrorCode, LkError, Result};
use crate::instance::{
    HostExecutionReceipt, INSTANCE_CONTRACT_VERSION, InstanceCreateReceipt, InstanceCreateRequest,
    InstanceDeleteRequest, InstanceEventRequest, InstanceFakeHostRequest, InstanceHistoryPage,
    InstanceHostRequest, InstanceId, InstanceInspection, InstanceOperationObservation,
    InstanceResumeRequest, InstanceStore, InstanceTransitionReceipt,
};
use crate::machine::{MAX_JSON_INPUT_BYTES, MAX_JSON_OUTPUT_BYTES};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

pub const RUNTIME_CONTRACT_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePolicy {
    pub maximum_request_bytes: u64,
    pub maximum_response_bytes: u64,
    pub maximum_application_bytes: u64,
    pub maximum_loaded_applications: u64,
    pub maximum_queued_requests: u64,
    pub maximum_active_transitions: u64,
    pub maximum_active_host_operations: u64,
    pub maximum_concurrent_compilations: u64,
    pub maximum_open_instance_stores: u64,
    pub maximum_compiled_unit_bytes: u64,
    pub maximum_cache_bytes: u64,
    pub maximum_profile_bytes: u64,
}

impl Default for RuntimePolicy {
    fn default() -> Self {
        Self {
            maximum_request_bytes: MAX_JSON_INPUT_BYTES as u64,
            maximum_response_bytes: MAX_JSON_OUTPUT_BYTES as u64,
            maximum_application_bytes: MAXIMUM_APPLICATION_ARTIFACT_BYTES as u64,
            maximum_loaded_applications: 1,
            maximum_queued_requests: 0,
            maximum_active_transitions: 1,
            maximum_active_host_operations: 1,
            maximum_concurrent_compilations: 1,
            maximum_open_instance_stores: 1,
            maximum_compiled_unit_bytes: 0,
            maximum_cache_bytes: 0,
            maximum_profile_bytes: 0,
        }
    }
}

impl RuntimePolicy {
    pub fn validate(self) -> Result<()> {
        let valid = self.maximum_request_bytes > 0
            && self.maximum_request_bytes <= MAX_JSON_INPUT_BYTES as u64
            && self.maximum_response_bytes > 0
            && self.maximum_response_bytes <= MAX_JSON_OUTPUT_BYTES as u64
            && self.maximum_application_bytes > 0
            && self.maximum_application_bytes <= MAXIMUM_APPLICATION_ARTIFACT_BYTES as u64
            && self.maximum_loaded_applications == 1
            && self.maximum_queued_requests == 0
            && self.maximum_active_transitions == 1
            && self.maximum_active_host_operations == 1
            && self.maximum_concurrent_compilations == 1
            && self.maximum_open_instance_stores == 1
            && self.maximum_compiled_unit_bytes == 0
            && self.maximum_cache_bytes == 0
            && self.maximum_profile_bytes == 0;
        if valid {
            Ok(())
        } else {
            Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "runtime policy exceeds the retained synchronous no-cache deployment contract",
            ))
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStage {
    RuntimeStartup,
    RequestDecode,
    PeerAuthentication,
    AuthorityResolution,
    ResourceAdmission,
    ApplicationRead,
    EnvelopeDecode,
    CanonicalReencode,
    ReleaseGraphValidation,
    ReleaseTests,
    ClosureFlattening,
    Lowering,
    CoreVerification,
    Execution,
    PublicValueMaterialization,
    InstanceStoreOpen,
    InstanceOpen,
    RecordChainValidation,
    DeterministicReplay,
    TransitionPreparation,
    StatePublication,
    GrantValidation,
    AdapterPreparation,
    HostAction,
    OutcomePublication,
    QueueDelay,
    CacheLookup,
    CachePopulation,
    CacheEviction,
    ResponseEncoding,
}

impl RuntimeStage {
    pub const ALL: [Self; 30] = [
        Self::RuntimeStartup,
        Self::RequestDecode,
        Self::PeerAuthentication,
        Self::AuthorityResolution,
        Self::ResourceAdmission,
        Self::ApplicationRead,
        Self::EnvelopeDecode,
        Self::CanonicalReencode,
        Self::ReleaseGraphValidation,
        Self::ReleaseTests,
        Self::ClosureFlattening,
        Self::Lowering,
        Self::CoreVerification,
        Self::Execution,
        Self::PublicValueMaterialization,
        Self::InstanceStoreOpen,
        Self::InstanceOpen,
        Self::RecordChainValidation,
        Self::DeterministicReplay,
        Self::TransitionPreparation,
        Self::StatePublication,
        Self::GrantValidation,
        Self::AdapterPreparation,
        Self::HostAction,
        Self::OutcomePublication,
        Self::QueueDelay,
        Self::CacheLookup,
        Self::CachePopulation,
        Self::CacheEviction,
        Self::ResponseEncoding,
    ];
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeStageObservation {
    pub count: u64,
    pub total_nanoseconds: u64,
    pub maximum_nanoseconds: u64,
    pub bytes: u64,
}

impl RuntimeStageObservation {
    fn add(&mut self, elapsed: Duration, bytes: u64) {
        let nanoseconds = elapsed_nanoseconds(elapsed);
        self.count = self.count.saturating_add(1);
        self.total_nanoseconds = self.total_nanoseconds.saturating_add(nanoseconds);
        self.maximum_nanoseconds = self.maximum_nanoseconds.max(nanoseconds);
        self.bytes = self.bytes.saturating_add(bytes);
    }

    fn add_nanoseconds(&mut self, nanoseconds: u64) {
        self.count = self.count.saturating_add(1);
        self.total_nanoseconds = self.total_nanoseconds.saturating_add(nanoseconds);
        self.maximum_nanoseconds = self.maximum_nanoseconds.max(nanoseconds);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeStageEntry {
    pub stage: RuntimeStage,
    pub observation: RuntimeStageObservation,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeResourceState {
    pub request_bytes_reserved: u64,
    pub response_bytes_reserved: u64,
    pub queued_requests: u64,
    pub active_transitions: u64,
    pub active_host_operations: u64,
    pub concurrent_compilations: u64,
    pub open_instance_stores: u64,
    pub loaded_application_count: u64,
    pub loaded_application_bytes: u64,
    pub compiled_unit_bytes: u64,
    pub cache_bytes: u64,
    pub profile_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCounters {
    pub requests: u64,
    pub rejected_admissions: u64,
    pub application_reads: u64,
    pub instance_operations: u64,
    pub adapter_operations: u64,
    pub compilations: u64,
    pub releases_decoded: u64,
    pub flattened_semantic_items: u64,
    pub replayed_records: u64,
    pub history_bytes_read: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeInspection {
    pub contract_version: u16,
    pub policy: RuntimePolicy,
    pub resources: RuntimeResourceState,
    pub counters: RuntimeCounters,
    pub stages: Vec<RuntimeStageEntry>,
    pub supported_topologies: Vec<&'static str>,
    pub supported_adapters: Vec<&'static str>,
    pub omissions: Vec<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeInterfaceOrientation {
    pub interface: HostInterface,
    pub identity: HostInterfaceId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeOrientation {
    pub runtime_contract_version: u16,
    pub application_contract_version: u16,
    pub instance_contract_version: u16,
    pub interfaces: Vec<RuntimeInterfaceOrientation>,
    pub topologies: Vec<&'static str>,
    pub adapters: Vec<&'static str>,
    pub default_policy: RuntimePolicy,
    pub exact_roots: Vec<&'static str>,
    pub expand_with: Vec<&'static str>,
}

pub struct RuntimeKernel {
    policy: RuntimePolicy,
    store: Option<InstanceStore>,
    resources: RuntimeResourceState,
    counters: RuntimeCounters,
    stages: BTreeMap<RuntimeStage, RuntimeStageObservation>,
}

impl RuntimeKernel {
    pub fn new(policy: RuntimePolicy) -> Result<Self> {
        let started = Instant::now();
        policy.validate()?;
        let mut kernel = Self {
            policy,
            store: None,
            resources: RuntimeResourceState::default(),
            counters: RuntimeCounters {
                requests: 0,
                rejected_admissions: 0,
                application_reads: 0,
                instance_operations: 0,
                adapter_operations: 0,
                compilations: 0,
                releases_decoded: 0,
                flattened_semantic_items: 0,
                replayed_records: 0,
                history_bytes_read: 0,
                cache_hits: 0,
                cache_misses: 0,
                cache_evictions: 0,
            },
            stages: BTreeMap::new(),
        };
        kernel.record(RuntimeStage::RuntimeStartup, started.elapsed(), 0);
        Ok(kernel)
    }

    pub fn open_instance_store(root: &Path, policy: RuntimePolicy) -> Result<Self> {
        let mut kernel = Self::new(policy)?;
        let started = Instant::now();
        let store = InstanceStore::open(root)?;
        kernel.store = Some(store);
        kernel.resources.open_instance_stores = 1;
        kernel.record(RuntimeStage::InstanceStoreOpen, started.elapsed(), 0);
        Ok(kernel)
    }

    pub fn inspect_application_path(&mut self, path: &Path) -> Result<ApplicationInspection> {
        let bytes = self.read_application(path)?;
        let mut observation = ApplicationLoadObservation::default();
        let result = application::inspect_observed(&bytes, &mut observation);
        self.record_application_observation(observation);
        result
    }

    pub fn test_application_path(&mut self, path: &Path) -> Result<ApplicationTestReport> {
        let bytes = self.read_application(path)?;
        let mut observation = ApplicationLoadObservation::default();
        let result = application::test_observed(&bytes, &mut observation);
        self.record_application_observation(observation);
        result
    }

    pub fn run_application_path(
        &mut self,
        path: &Path,
        invocation: &ApplicationInvocation,
    ) -> Result<ApplicationRunReceipt> {
        self.admit(invocation)?;
        self.reserve_compilation()?;
        let bytes = match self.read_application(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.release_compilation();
                return Err(error);
            }
        };
        let mut application_observation = ApplicationLoadObservation::default();
        let result = application::run_observed(&bytes, invocation, &mut application_observation);
        self.release_compilation();
        self.record_application_observation(application_observation);
        if let Ok(receipt) = &result {
            self.record_nanoseconds(RuntimeStage::Lowering, receipt.result.lowering_nanoseconds);
            self.record_nanoseconds(
                RuntimeStage::CoreVerification,
                receipt.result.core_verification_nanoseconds,
            );
            self.record_nanoseconds(RuntimeStage::Execution, receipt.result.execute_nanoseconds);
            self.record_nanoseconds(
                RuntimeStage::PublicValueMaterialization,
                receipt.result.public_value_nanoseconds,
            );
            self.counters.compilations = self.counters.compilations.saturating_add(1);
        }
        result
    }

    pub fn run_stream_application_path(&mut self, path: &Path, input: &[u8]) -> Result<Vec<u8>> {
        if input.len() as u64 > self.policy.maximum_request_bytes {
            self.counters.rejected_admissions = self.counters.rejected_admissions.saturating_add(1);
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "runtime stream input exceeds deployment byte policy",
            ));
        }
        self.reserve_compilation()?;
        let bytes = match self.read_application(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.release_compilation();
                return Err(error);
            }
        };
        let started = Instant::now();
        let result = application::run_stream(&bytes, input);
        self.release_compilation();
        self.counters.compilations = self.counters.compilations.saturating_add(1);
        self.record(
            RuntimeStage::Execution,
            started.elapsed(),
            input.len() as u64,
        );
        result
    }

    pub fn create_from_path(
        &mut self,
        request: &InstanceCreateRequest,
        application_path: &Path,
    ) -> Result<InstanceCreateReceipt> {
        self.admit(request)?;
        let bytes = self.read_application(application_path)?;
        self.transition_operation(|store, observation| {
            store.create_observed(request, &bytes, observation)
        })
    }

    pub fn validate_event(
        &mut self,
        request: &InstanceEventRequest,
    ) -> Result<InstanceTransitionReceipt> {
        self.admit(request)?;
        self.transition_operation(|store, observation| {
            store.validate_event_observed(request, observation)
        })
    }

    pub fn apply_event(
        &mut self,
        request: &InstanceEventRequest,
    ) -> Result<InstanceTransitionReceipt> {
        self.admit(request)?;
        self.transition_operation(|store, observation| {
            store.apply_event_observed(request, observation)
        })
    }

    pub fn validate_resume(
        &mut self,
        request: &InstanceResumeRequest,
    ) -> Result<InstanceTransitionReceipt> {
        self.admit(request)?;
        self.transition_operation(|store, observation| {
            store.validate_resume_observed(request, observation)
        })
    }

    pub fn resume(&mut self, request: &InstanceResumeRequest) -> Result<InstanceTransitionReceipt> {
        self.admit(request)?;
        self.transition_operation(|store, observation| store.resume_observed(request, observation))
    }

    pub fn execute_host(&mut self, request: &InstanceHostRequest) -> Result<HostExecutionReceipt> {
        self.admit(request)?;
        self.host_operation(|store, observation| store.execute_host_observed(request, observation))
    }

    pub fn record_fake_outcome(
        &mut self,
        request: &InstanceFakeHostRequest,
    ) -> Result<HostExecutionReceipt> {
        self.admit(request)?;
        self.host_operation(|store, observation| {
            store.record_fake_outcome_observed(request, observation)
        })
    }

    pub fn inspect_instance(&mut self, instance: InstanceId) -> Result<InstanceInspection> {
        self.simple_instance_operation(|store, observation| {
            store.inspect_observed(instance, observation)
        })
    }

    pub fn history(
        &mut self,
        instance: InstanceId,
        start: u64,
        limit: usize,
    ) -> Result<InstanceHistoryPage> {
        self.simple_instance_operation(|store, observation| {
            store.history_observed(instance, start, limit, observation)
        })
    }

    pub fn delete(&mut self, request: InstanceDeleteRequest) -> Result<InstanceInspection> {
        self.admit(&request)?;
        self.simple_instance_operation(|store, observation| {
            store.delete_observed(request, observation)
        })
    }

    pub fn observe_request_decode(&mut self, elapsed: Duration, bytes: usize) {
        self.record(RuntimeStage::RequestDecode, elapsed, bytes as u64);
    }

    pub fn observe_response_encoding(&mut self, elapsed: Duration, bytes: usize) -> Result<()> {
        if bytes as u64 > self.policy.maximum_response_bytes {
            self.counters.rejected_admissions = self.counters.rejected_admissions.saturating_add(1);
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "runtime response exceeds deployment byte policy",
            ));
        }
        self.record(RuntimeStage::ResponseEncoding, elapsed, bytes as u64);
        Ok(())
    }

    pub fn inspection(&self) -> RuntimeInspection {
        RuntimeInspection {
            contract_version: RUNTIME_CONTRACT_VERSION,
            policy: self.policy,
            resources: self.resources,
            counters: self.counters.clone(),
            stages: RuntimeStage::ALL
                .into_iter()
                .map(|stage| RuntimeStageEntry {
                    stage,
                    observation: self.stages.get(&stage).copied().unwrap_or_default(),
                })
                .collect(),
            supported_topologies: vec!["one_shot", "foreground_session"],
            supported_adapters: vec![
                "application_activation.production",
                "application_activation.deterministic_fake",
                "immutable_blob.production",
                "immutable_blob.deterministic_fake",
            ],
            omissions: vec![
                "no retained application or compiled-unit cache",
                "no runtime request queue or worker pool",
                "no persistent profiles",
                "resident-set size is observation only and is not sampled",
                "open files and temporary publication bytes remain owned by exact artifact adapters",
                "one synchronous store lock serializes instance operations",
            ],
        }
    }

    pub fn orientation(&self) -> RuntimeOrientation {
        RuntimeOrientation {
            runtime_contract_version: RUNTIME_CONTRACT_VERSION,
            application_contract_version: APPLICATION_CONTRACT_VERSION,
            instance_contract_version: INSTANCE_CONTRACT_VERSION,
            interfaces: [
                HostInterface::ApplicationActivation,
                HostInterface::ImmutableBlob,
            ]
            .into_iter()
            .map(|interface| RuntimeInterfaceOrientation {
                interface,
                identity: interface.identity(),
            })
            .collect(),
            topologies: vec!["one_shot", "foreground_session"],
            adapters: vec![
                "application_activation.production",
                "application_activation.deterministic_fake",
                "immutable_blob.production",
                "immutable_blob.deterministic_fake",
            ],
            default_policy: self.policy,
            exact_roots: vec!["application_artifact_path", "instance_store_path"],
            expand_with: vec![
                "lkjscript app inspect --artifact FILE",
                "lkjscript instance inspect --store DIRECTORY --instance HEX",
                "lkjscript runtime inspect --store DIRECTORY",
                "lkjscript runtime help",
            ],
        }
    }

    fn read_application(&mut self, path: &Path) -> Result<Vec<u8>> {
        let started = Instant::now();
        let result = application::read_file(path);
        let bytes = result.as_ref().map_or(0, Vec::len);
        self.record(
            RuntimeStage::ApplicationRead,
            started.elapsed(),
            bytes as u64,
        );
        if bytes as u64 > self.policy.maximum_application_bytes {
            self.counters.rejected_admissions = self.counters.rejected_admissions.saturating_add(1);
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "application exceeds runtime deployment byte policy",
            ));
        }
        self.counters.application_reads = self.counters.application_reads.saturating_add(1);
        result
    }

    fn admit<T: Serialize>(&mut self, request: &T) -> Result<()> {
        let started = Instant::now();
        let bytes = serde_json::to_vec(request).map_err(|error| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                format!("cannot account runtime request: {error}"),
            )
        })?;
        if bytes.len() as u64 > self.policy.maximum_request_bytes {
            self.counters.rejected_admissions = self.counters.rejected_admissions.saturating_add(1);
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "runtime request exceeds deployment byte policy",
            ));
        }
        self.counters.requests = self.counters.requests.saturating_add(1);
        self.resources.request_bytes_reserved = bytes.len() as u64;
        self.record(
            RuntimeStage::ResourceAdmission,
            started.elapsed(),
            bytes.len() as u64,
        );
        self.resources.request_bytes_reserved = 0;
        Ok(())
    }

    fn reserve_compilation(&mut self) -> Result<()> {
        if self.resources.concurrent_compilations >= self.policy.maximum_concurrent_compilations {
            self.counters.rejected_admissions = self.counters.rejected_admissions.saturating_add(1);
            return Err(LkError::new(
                ErrorCode::AuthorityBusy,
                "runtime compilation capacity is busy",
            ));
        }
        self.resources.concurrent_compilations += 1;
        Ok(())
    }

    fn release_compilation(&mut self) {
        self.resources.concurrent_compilations =
            self.resources.concurrent_compilations.saturating_sub(1);
    }

    fn transition_operation<T>(
        &mut self,
        operation: impl FnOnce(&InstanceStore, &mut InstanceOperationObservation) -> Result<T>,
    ) -> Result<T> {
        if self.resources.active_transitions >= self.policy.maximum_active_transitions {
            self.counters.rejected_admissions = self.counters.rejected_admissions.saturating_add(1);
            return Err(LkError::new(
                ErrorCode::AuthorityBusy,
                "runtime transition capacity is busy",
            ));
        }
        self.resources.active_transitions += 1;
        if let Err(error) = self.reserve_compilation() {
            self.resources.active_transitions = self.resources.active_transitions.saturating_sub(1);
            return Err(error);
        }
        let mut observation = InstanceOperationObservation::default();
        let result = self
            .store()
            .and_then(|store| operation(store, &mut observation));
        self.release_compilation();
        self.resources.active_transitions = self.resources.active_transitions.saturating_sub(1);
        self.counters.instance_operations = self.counters.instance_operations.saturating_add(1);
        self.counters.compilations = self.counters.compilations.saturating_add(1);
        self.record_instance_observation(observation);
        result
    }

    fn host_operation<T>(
        &mut self,
        operation: impl FnOnce(&InstanceStore, &mut InstanceOperationObservation) -> Result<T>,
    ) -> Result<T> {
        if self.resources.active_host_operations >= self.policy.maximum_active_host_operations {
            self.counters.rejected_admissions = self.counters.rejected_admissions.saturating_add(1);
            return Err(LkError::new(
                ErrorCode::AuthorityBusy,
                "runtime host-operation capacity is busy",
            ));
        }
        self.resources.active_host_operations += 1;
        let mut observation = InstanceOperationObservation::default();
        let result = self
            .store()
            .and_then(|store| operation(store, &mut observation));
        self.resources.active_host_operations =
            self.resources.active_host_operations.saturating_sub(1);
        self.counters.instance_operations = self.counters.instance_operations.saturating_add(1);
        self.counters.adapter_operations = self.counters.adapter_operations.saturating_add(1);
        self.record_instance_observation(observation);
        result
    }

    fn simple_instance_operation<T>(
        &mut self,
        operation: impl FnOnce(&InstanceStore, &mut InstanceOperationObservation) -> Result<T>,
    ) -> Result<T> {
        let mut observation = InstanceOperationObservation::default();
        let result = self
            .store()
            .and_then(|store| operation(store, &mut observation));
        self.counters.instance_operations = self.counters.instance_operations.saturating_add(1);
        self.record_instance_observation(observation);
        result
    }

    fn store(&self) -> Result<&InstanceStore> {
        self.store.as_ref().ok_or_else(|| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "runtime kernel was not constructed with an instance store",
            )
        })
    }

    fn record_instance_observation(&mut self, observation: InstanceOperationObservation) {
        self.record_observed_nanoseconds(
            RuntimeStage::ApplicationRead,
            observation.application_read_nanoseconds,
            observation.application_bytes,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::EnvelopeDecode,
            observation.envelope_decode_nanoseconds,
            observation.application_bytes,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::CanonicalReencode,
            observation.canonical_reencode_nanoseconds,
            observation.application_bytes,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::ReleaseGraphValidation,
            observation.release_graph_validation_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::ClosureFlattening,
            observation.closure_flattening_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::ReleaseTests,
            observation.release_tests_nanoseconds,
            0,
        );
        self.counters.releases_decoded = self
            .counters
            .releases_decoded
            .saturating_add(observation.release_count);
        self.counters.flattened_semantic_items = self
            .counters
            .flattened_semantic_items
            .saturating_add(observation.flattened_semantic_items);
        self.counters.replayed_records = self
            .counters
            .replayed_records
            .saturating_add(observation.replay_records);
        self.counters.history_bytes_read = self
            .counters
            .history_bytes_read
            .saturating_add(observation.history_bytes);
        self.record_observed_nanoseconds(
            RuntimeStage::InstanceOpen,
            observation.instance_open_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::RecordChainValidation,
            observation.record_chain_validation_nanoseconds,
            observation.history_bytes,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::DeterministicReplay,
            observation.replay_nanoseconds,
            observation.replay_records,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::TransitionPreparation,
            observation.transition_preparation_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::StatePublication,
            observation.state_publication_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::GrantValidation,
            observation.grant_validation_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::AdapterPreparation,
            observation.adapter_preparation_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::HostAction,
            observation.host_action_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::OutcomePublication,
            observation.outcome_publication_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::Lowering,
            observation.lowering_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::CoreVerification,
            observation.core_verification_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::Execution,
            observation.execution_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::PublicValueMaterialization,
            observation.public_value_nanoseconds,
            0,
        );
    }

    fn record_application_observation(&mut self, observation: ApplicationLoadObservation) {
        self.record_observed_nanoseconds(
            RuntimeStage::EnvelopeDecode,
            observation.envelope_decode_nanoseconds,
            observation.application_bytes,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::CanonicalReencode,
            observation.canonical_reencode_nanoseconds,
            observation.application_bytes,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::ReleaseGraphValidation,
            observation.release_graph_validation_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::ClosureFlattening,
            observation.closure_flattening_nanoseconds,
            0,
        );
        self.record_observed_nanoseconds(
            RuntimeStage::ReleaseTests,
            observation.release_tests_nanoseconds,
            0,
        );
        self.counters.releases_decoded = self
            .counters
            .releases_decoded
            .saturating_add(observation.release_count);
        self.counters.flattened_semantic_items = self
            .counters
            .flattened_semantic_items
            .saturating_add(observation.flattened_semantic_items);
    }

    fn record(&mut self, stage: RuntimeStage, elapsed: Duration, bytes: u64) {
        self.stages.entry(stage).or_default().add(elapsed, bytes);
    }

    fn record_nanoseconds(&mut self, stage: RuntimeStage, nanoseconds: u64) {
        self.stages
            .entry(stage)
            .or_default()
            .add_nanoseconds(nanoseconds);
    }

    fn record_observed_nanoseconds(&mut self, stage: RuntimeStage, nanoseconds: u64, bytes: u64) {
        if nanoseconds == 0 && bytes == 0 {
            return;
        }
        let observation = self.stages.entry(stage).or_default();
        observation.count = observation.count.saturating_add(1);
        observation.total_nanoseconds = observation.total_nanoseconds.saturating_add(nanoseconds);
        observation.maximum_nanoseconds = observation.maximum_nanoseconds.max(nanoseconds);
        observation.bytes = observation.bytes.saturating_add(bytes);
    }
}

fn elapsed_nanoseconds(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_has_no_implicit_queue_cache_or_profile_authority() {
        let kernel = RuntimeKernel::new(RuntimePolicy::default()).expect("kernel");
        let inspection = kernel.inspection();
        assert_eq!(inspection.policy.maximum_queued_requests, 0);
        assert_eq!(inspection.resources.cache_bytes, 0);
        assert_eq!(inspection.resources.profile_bytes, 0);
        assert_eq!(inspection.stages.len(), RuntimeStage::ALL.len());
    }

    #[test]
    fn unsupported_retained_cache_policy_rejects() {
        let policy = RuntimePolicy {
            maximum_cache_bytes: 1,
            ..RuntimePolicy::default()
        };
        assert_eq!(
            RuntimeKernel::new(policy).err().expect("rejection").code,
            ErrorCode::PolicyExceeded
        );
    }

    #[test]
    fn synchronous_capacities_are_exact_busy_and_released_after_failure() {
        for policy in [
            RuntimePolicy {
                maximum_loaded_applications: 2,
                ..RuntimePolicy::default()
            },
            RuntimePolicy {
                maximum_active_transitions: 2,
                ..RuntimePolicy::default()
            },
            RuntimePolicy {
                maximum_active_host_operations: 2,
                ..RuntimePolicy::default()
            },
            RuntimePolicy {
                maximum_concurrent_compilations: 2,
                ..RuntimePolicy::default()
            },
        ] {
            assert_eq!(
                RuntimeKernel::new(policy)
                    .err()
                    .expect("unsupported capacity rejection")
                    .code,
                ErrorCode::PolicyExceeded
            );
        }

        let mut kernel = RuntimeKernel::new(RuntimePolicy::default()).expect("kernel");
        kernel.resources.active_transitions = 1;
        assert_eq!(
            kernel
                .transition_operation::<()>(|_, _| unreachable!())
                .expect_err("busy transition")
                .code,
            ErrorCode::AuthorityBusy
        );
        kernel.resources.active_transitions = 0;

        kernel.resources.concurrent_compilations = 1;
        assert_eq!(
            kernel
                .transition_operation::<()>(|_, _| unreachable!())
                .expect_err("busy compilation")
                .code,
            ErrorCode::AuthorityBusy
        );
        assert_eq!(kernel.resources.active_transitions, 0);
        kernel.resources.concurrent_compilations = 0;

        kernel.resources.active_host_operations = 1;
        assert_eq!(
            kernel
                .host_operation::<()>(|_, _| unreachable!())
                .expect_err("busy host operation")
                .code,
            ErrorCode::AuthorityBusy
        );
        kernel.resources.active_host_operations = 0;

        assert_eq!(
            kernel
                .transition_operation::<()>(|_, _| unreachable!())
                .expect_err("store-less transition")
                .code,
            ErrorCode::ProtocolMalformed
        );
        assert_eq!(kernel.resources.active_transitions, 0);
        assert_eq!(kernel.resources.concurrent_compilations, 0);
        assert_eq!(
            kernel
                .host_operation::<()>(|_, _| unreachable!())
                .expect_err("store-less host operation")
                .code,
            ErrorCode::ProtocolMalformed
        );
        assert_eq!(kernel.resources.active_host_operations, 0);
        assert_eq!(kernel.counters.rejected_admissions, 3);
    }

    #[test]
    fn deployment_byte_limits_accept_exact_global_bounds_and_reject_one_over() {
        let exact = RuntimePolicy {
            maximum_request_bytes: MAX_JSON_INPUT_BYTES as u64,
            maximum_response_bytes: MAX_JSON_OUTPUT_BYTES as u64,
            maximum_application_bytes: MAXIMUM_APPLICATION_ARTIFACT_BYTES as u64,
            ..RuntimePolicy::default()
        };
        RuntimeKernel::new(exact).expect("exact deployment bounds");
        for policy in [
            RuntimePolicy {
                maximum_request_bytes: MAX_JSON_INPUT_BYTES as u64 + 1,
                ..exact
            },
            RuntimePolicy {
                maximum_response_bytes: MAX_JSON_OUTPUT_BYTES as u64 + 1,
                ..exact
            },
            RuntimePolicy {
                maximum_application_bytes: MAXIMUM_APPLICATION_ARTIFACT_BYTES as u64 + 1,
                ..exact
            },
        ] {
            assert_eq!(
                RuntimeKernel::new(policy)
                    .err()
                    .expect("one-over rejection")
                    .code,
                ErrorCode::PolicyExceeded
            );
        }
    }
}
