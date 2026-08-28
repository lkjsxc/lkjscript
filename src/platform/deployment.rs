//! Strict standalone artifact-10 deployment and normalized resident execution.

use super::compiler::{MAXIMUM_ARTIFACT_BUNDLE_BYTES, load_artifact};
use super::configuration::{ConfigurationObservation, ConfigurationStore, ConfigurationValue};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::RunPolicy;
use super::execution::normalized::{
    NormalizedAdapterDescriptor, NormalizedDeploymentGrant, NormalizedDeploymentResourcePolicy,
    NormalizedGrantAuthorityRevision, NormalizedGrantLimit, NormalizedHttpApplication,
    NormalizedPreparedDeployment, NormalizedProgram, NormalizedResidentDeployment,
    NormalizedRunPolicy, NormalizedSharingDomain, NormalizedWorkerApplication,
};
use super::http::{
    HttpDispatchObservation, HttpLimits, HttpRequest, HttpResponse, HttpServerReceipt,
};
use super::kernel::Name;
use super::object::ObjectLimits;
use super::package::RunnerKind;
use super::queue::QueueLimits;
use super::runtime::{ResidentLimits, ResidentObservation, ShutdownReceipt};
use super::secrets::{EnvironmentSecretBinding, SecretCatalog};
use super::security::PasswordHashPolicy;
use super::stream::StreamLimits;
use super::worker::{WorkerLimits, WorkerReceipt};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Handle;

pub const DEPLOYMENT_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_DEPLOYMENT_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_DEPLOYMENT_GRANTS: usize = 1_024;
pub(crate) const STARTER_HTTP_DESCRIPTOR_PATH: &str = "service.deployment.json";
pub(crate) const STARTER_HTTP_ARTIFACT_PATH: &str = "generated/application.lkja";
pub(crate) const STARTER_HTTP_ARTIFACT_DIRECTORY: &str = "generated";
pub(crate) const STARTER_HTTP_TARGET: &str = "serve";
pub(crate) const STARTER_HTTP_LISTENER: &str = "127.0.0.1:0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeploymentAuthorityRevision([u8; 32]);

impl DeploymentAuthorityRevision {
    fn generate() -> Result<Self, Diagnostic> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "deployment_authority_entropy",
                "operating-system entropy is unavailable for starter deployment authority",
            )
        })?;
        if bytes == [0; 32] {
            bytes[31] = 1;
        }
        Ok(Self(bytes))
    }

    fn encode(self) -> String {
        super::semantic_id::encode_hex(&self.0)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentDescriptor {
    pub contract_version: u16,
    pub artifact: String,
    pub target: String,
    pub listen: Option<String>,
    pub runtime: ResidentLimits,
    pub execution: RunPolicy,
    pub http: Option<HttpLimits>,
    pub worker: Option<WorkerLimits>,
    pub streams: StreamLimits,
    pub configuration: BTreeMap<String, ConfigurationValue>,
    pub secrets: Vec<EnvironmentSecretBinding>,
    pub grants: Vec<DeploymentGrant>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentGrant {
    pub requirement: String,
    pub sharing_domain: String,
    pub authority_revision: String,
    pub adapter: AdapterDescriptor,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AdapterDescriptor {
    Configuration,
    WallClock,
    SecureRandom,
    Identifier,
    PasswordHash {
        policy: PasswordHashPolicy,
    },
    SecretVerifier {
        secret: String,
        maximum_candidate_bytes: usize,
    },
    ByteStream,
    Postgres {
        connection_secret: String,
        maximum_connections: usize,
        maximum_wait_milliseconds: u64,
        statement_timeout_milliseconds: u64,
    },
    ObjectMemory {
        prefix: String,
        limits: ObjectLimits,
    },
    ObjectLocal {
        root: String,
        prefix: String,
        limits: ObjectLimits,
    },
    ObjectS3 {
        endpoint: String,
        region: String,
        bucket: String,
        prefix: String,
        allow_http: bool,
        path_style: bool,
        access_key_secret: String,
        secret_key_secret: String,
        limits: ObjectLimits,
    },
    DurableQueueMemory {
        limits: QueueLimits,
    },
    DurableQueuePostgres {
        connection_secret: String,
        namespace: String,
        maximum_connections: usize,
        maximum_wait_milliseconds: u64,
        statement_timeout_milliseconds: u64,
        limits: QueueLimits,
    },
}

pub(crate) fn starter_http_deployment() -> Result<DeploymentDescriptor, Diagnostic> {
    let descriptor = DeploymentDescriptor {
        contract_version: DEPLOYMENT_CONTRACT_VERSION,
        artifact: STARTER_HTTP_ARTIFACT_PATH.to_owned(),
        target: STARTER_HTTP_TARGET.to_owned(),
        listen: Some(STARTER_HTTP_LISTENER.to_owned()),
        runtime: ResidentLimits {
            maximum_concurrent_tasks: 16,
            maximum_queued_tasks: 64,
            request_deadline_milliseconds: 30_000,
            shutdown_grace_milliseconds: 30_000,
            cancellation_grace_milliseconds: 5_000,
            ..ResidentLimits::default()
        },
        execution: RunPolicy::default(),
        http: Some(HttpLimits {
            maximum_request_body_bytes: 8 * 1024 * 1024,
            maximum_response_body_bytes: 4 * 1024 * 1024,
            maximum_header_bytes: 32 * 1024,
            maximum_headers: 128,
            ..HttpLimits::default()
        }),
        worker: None,
        streams: StreamLimits {
            maximum_chunk_bytes: 64 * 1024,
            maximum_buffered_chunks: 8,
            maximum_total_bytes: 64 * 1024 * 1024,
            maximum_live_streams: 1_024,
        },
        configuration: BTreeMap::new(),
        secrets: Vec::new(),
        grants: vec![DeploymentGrant {
            requirement: "streams".to_owned(),
            sharing_domain: "http-request-streams".to_owned(),
            authority_revision: DeploymentAuthorityRevision::generate()?.encode(),
            adapter: AdapterDescriptor::ByteStream,
        }],
    };
    validate_descriptor(&descriptor)?;
    Ok(descriptor)
}

pub(crate) fn encode_deployment(descriptor: &DeploymentDescriptor) -> Result<Vec<u8>, Diagnostic> {
    validate_descriptor(descriptor)?;
    let mut bytes = serde_json::to_vec_pretty(descriptor).map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Infrastructure,
            "deployment_encode",
            format!("deployment descriptor could not be encoded: {error}"),
        )
    })?;
    bytes.push(b'\n');
    if bytes.len() > MAXIMUM_DEPLOYMENT_BYTES {
        return Err(deployment_error(
            "deployment_too_large",
            format!(
                "deployment descriptor has {} bytes; the limit is {MAXIMUM_DEPLOYMENT_BYTES}",
                bytes.len()
            ),
        ));
    }
    let _ = decode_deployment(&bytes)?;
    Ok(bytes)
}

impl AdapterDescriptor {
    fn kind(&self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::WallClock => "wall-clock",
            Self::SecureRandom => "secure-random",
            Self::Identifier => "identifier",
            Self::PasswordHash { .. } => "password-hash",
            Self::SecretVerifier { .. } => "secret-verifier",
            Self::ByteStream => "byte-stream",
            Self::Postgres { .. } => "postgres",
            Self::ObjectMemory { .. } => "object-memory",
            Self::ObjectLocal { .. } => "object-local",
            Self::ObjectS3 { .. } => "object-s3",
            Self::DurableQueueMemory { .. } => "durable-queue-memory",
            Self::DurableQueuePostgres { .. } => "durable-queue-postgres",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentObservation {
    pub contract_version: u16,
    pub artifact_digest: String,
    pub target: String,
    pub runner: String,
    pub listen: Option<String>,
    pub configuration: ConfigurationObservation,
    pub secret_names: Vec<String>,
    pub grants: BTreeMap<String, String>,
}

#[derive(Clone)]
pub struct PreparedDeployment {
    descriptor: DeploymentDescriptor,
    program: Arc<NormalizedProgram>,
    deployment: NormalizedPreparedDeployment,
    observation: DeploymentObservation,
}

impl std::fmt::Debug for PreparedDeployment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedDeployment")
            .field("observation", &self.observation)
            .finish()
    }
}

impl PreparedDeployment {
    pub fn load(path: &Path, runtime: Handle) -> Result<Self, Diagnostic> {
        let descriptor_bytes = read_bounded(
            path,
            MAXIMUM_DEPLOYMENT_BYTES as u64,
            "deployment descriptor",
        )?;
        let descriptor = decode_deployment(&descriptor_bytes)?;
        let directory = path.parent().unwrap_or_else(|| Path::new("."));
        let artifact_path = resolve_relative(directory, &descriptor.artifact, "artifact")?;
        let artifact_bytes = read_bounded(
            &artifact_path,
            MAXIMUM_ARTIFACT_BUNDLE_BYTES,
            "component artifact",
        )?;
        let artifact = load_artifact(&artifact_bytes)?;
        let artifact_digest = artifact.bundle_digest.to_string();
        let program = Arc::new(NormalizedProgram::prepare(artifact)?);

        // Resolve target, runner, exact requirements, adapter kinds, and grant closure before
        // reading any named secret from the process environment.
        validate_program_descriptor(&descriptor, &program)?;
        let secrets = SecretCatalog::from_environment(&descriptor.secrets)?;
        Self::prepare(
            descriptor,
            program,
            artifact_digest,
            directory,
            runtime,
            secrets,
        )
    }

    fn prepare(
        descriptor: DeploymentDescriptor,
        program: Arc<NormalizedProgram>,
        artifact_digest: String,
        deployment_directory: &Path,
        runtime: Handle,
        secrets: SecretCatalog,
    ) -> Result<Self, Diagnostic> {
        let target_name = Name::new(descriptor.target.clone())?;
        let target = program.root_target(&target_name).cloned().ok_or_else(|| {
            deployment_error(
                "deployment_target_missing",
                "deployment names no exact root-package artifact target",
            )
        })?;
        let component = program
            .components
            .get(target.component.0 as usize)
            .ok_or_else(|| {
                deployment_error(
                    "deployment_component_missing",
                    "selected target component escaped the exact artifact table",
                )
            })?;
        let configuration = ConfigurationStore::observe_values(&descriptor.configuration)?;
        let mut supplied = descriptor
            .grants
            .iter()
            .map(|grant| (grant.requirement.as_str(), grant))
            .collect::<BTreeMap<_, _>>();
        let mut grants = Vec::with_capacity(component.requirements.len());
        let mut observed_grants = BTreeMap::new();
        for requirement_index in component.requirements.iter().copied() {
            let requirement = program
                .requirements
                .get(requirement_index.0 as usize)
                .ok_or_else(|| {
                    deployment_error(
                        "deployment_requirement_missing",
                        "component requirement escaped the exact artifact table",
                    )
                })?;
            let alias = requirement.name.as_str();
            let declared = supplied.remove(alias).ok_or_else(|| {
                deployment_error(
                    "deployment_grant_missing",
                    format!("component requirement '{alias}' has no deployment grant"),
                )
            })?;
            grants.push(NormalizedDeploymentGrant {
                requirement: requirement.reference,
                sharing_domain: NormalizedSharingDomain::new(declared.sharing_domain.clone())?,
                authority_revision: NormalizedGrantAuthorityRevision::of(
                    declared.authority_revision.as_bytes(),
                ),
                limits: requirement
                    .limits
                    .iter()
                    .map(|limit| {
                        (
                            limit.name.clone(),
                            NormalizedGrantLimit {
                                maximum: limit.maximum,
                                unit: limit.unit,
                            },
                        )
                    })
                    .collect(),
                adapter: normalized_adapter(&declared.adapter, &descriptor.configuration),
            });
            observed_grants.insert(alias.to_owned(), declared.adapter.kind().to_owned());
        }
        if let Some((alias, _)) = supplied.into_iter().next() {
            return Err(deployment_error(
                "deployment_grant_foreign",
                format!("deployment grants undeclared component requirement '{alias}'"),
            ));
        }
        let deployment = NormalizedPreparedDeployment::prepare_with_host(
            &program,
            target_name,
            grants,
            NormalizedDeploymentResourcePolicy {
                streams: descriptor.streams.clone(),
            },
            &secrets,
            deployment_directory,
            runtime,
        )?;
        let observation = DeploymentObservation {
            contract_version: DEPLOYMENT_CONTRACT_VERSION,
            artifact_digest,
            target: descriptor.target.clone(),
            runner: format!("{:?}", target.runner).to_ascii_lowercase(),
            listen: descriptor.listen.clone(),
            configuration,
            secret_names: secrets.names(),
            grants: observed_grants,
        };
        Ok(Self {
            descriptor,
            program,
            deployment,
            observation,
        })
    }

    pub fn observe_redacted(&self) -> &DeploymentObservation {
        &self.observation
    }

    pub fn listen(&self) -> Option<&str> {
        self.descriptor.listen.as_deref()
    }

    fn resident(&self) -> Result<NormalizedResidentDeployment, Diagnostic> {
        NormalizedResidentDeployment::prepare(
            Arc::clone(&self.program),
            self.deployment.clone(),
            self.descriptor.runtime.clone(),
            normalized_run_policy(self.descriptor.execution),
        )
    }

    pub fn http_application(&self) -> Result<PreparedHttpApplication, Diagnostic> {
        let limits = self.descriptor.http.clone().ok_or_else(|| {
            deployment_error(
                "deployment_http_missing",
                "HTTP target requires an HTTP limits descriptor",
            )
        })?;
        NormalizedHttpApplication::new(self.resident()?, limits).map(PreparedHttpApplication)
    }

    pub fn worker_application(&self) -> Result<PreparedWorkerApplication, Diagnostic> {
        let limits = self.descriptor.worker.clone().ok_or_else(|| {
            deployment_error(
                "deployment_worker_missing",
                "worker target requires a worker limits descriptor",
            )
        })?;
        NormalizedWorkerApplication::new(self.resident()?, limits).map(PreparedWorkerApplication)
    }
}

#[derive(Clone)]
pub struct PreparedHttpApplication(NormalizedHttpApplication);

impl PreparedHttpApplication {
    pub async fn dispatch(
        &self,
        request: HttpRequest,
    ) -> Result<(HttpResponse, HttpDispatchObservation), super::execution::ExecutionError> {
        self.0.dispatch(request).await
    }

    pub async fn serve(
        self,
        listener: TcpListener,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<HttpServerReceipt, Diagnostic> {
        self.0.serve(listener, shutdown).await
    }

    pub fn observe_resident(&self) -> ResidentObservation {
        self.0.resident().observe()
    }

    pub async fn shutdown(&self) -> ShutdownReceipt {
        self.0.resident().shutdown().await
    }
}

#[derive(Clone)]
pub struct PreparedWorkerApplication(NormalizedWorkerApplication);

impl PreparedWorkerApplication {
    pub async fn run(
        self,
        shutdown: impl Future<Output = ()> + Send,
    ) -> Result<WorkerReceipt, Diagnostic> {
        self.0.run(shutdown).await
    }

    pub fn observe_resident(&self) -> ResidentObservation {
        self.0.resident().observe()
    }

    pub async fn shutdown(&self) -> ShutdownReceipt {
        self.0.resident().shutdown().await
    }
}

pub fn decode_deployment(bytes: &[u8]) -> Result<DeploymentDescriptor, Diagnostic> {
    if bytes.len() > MAXIMUM_DEPLOYMENT_BYTES {
        return Err(deployment_error(
            "deployment_too_large",
            format!(
                "deployment descriptor has {} bytes; the limit is {MAXIMUM_DEPLOYMENT_BYTES}",
                bytes.len()
            ),
        ));
    }
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let descriptor = DeploymentDescriptor::deserialize(&mut decoder).map_err(|error| {
        deployment_error(
            "deployment_json",
            format!("deployment descriptor is not strict JSON: {error}"),
        )
    })?;
    decoder.end().map_err(|error| {
        deployment_error(
            "deployment_trailing_json",
            format!("deployment descriptor has trailing input: {error}"),
        )
    })?;
    validate_descriptor(&descriptor)?;
    Ok(descriptor)
}

fn validate_descriptor(descriptor: &DeploymentDescriptor) -> Result<(), Diagnostic> {
    if descriptor.contract_version != DEPLOYMENT_CONTRACT_VERSION {
        return Err(deployment_error(
            "deployment_contract",
            format!(
                "deployment contract {} is not current contract {DEPLOYMENT_CONTRACT_VERSION}",
                descriptor.contract_version
            ),
        ));
    }
    validate_relative(&descriptor.artifact, "artifact")?;
    validate_name(&descriptor.target, "target")?;
    if descriptor.grants.len() > MAXIMUM_DEPLOYMENT_GRANTS {
        return Err(deployment_error(
            "deployment_grant_limit",
            format!("deployment has more than {MAXIMUM_DEPLOYMENT_GRANTS} grants"),
        ));
    }
    if let Some(listen) = &descriptor.listen
        && (listen.is_empty() || listen.len() > 512 || listen.contains('\0'))
    {
        return Err(deployment_error(
            "deployment_listener",
            "listener descriptor is empty, excessive, or contains NUL",
        ));
    }
    descriptor.runtime.validate()?;
    descriptor.streams.validate()?;
    if let Some(http) = &descriptor.http {
        http.validate()?;
    }
    if let Some(worker) = &descriptor.worker {
        worker.validate(descriptor.runtime.maximum_concurrent_tasks)?;
    }
    let mut requirements = BTreeSet::new();
    for grant in &descriptor.grants {
        validate_name(&grant.requirement, "requirement")?;
        validate_name(&grant.sharing_domain, "sharing domain")?;
        validate_digest(&grant.authority_revision, "authority revision")?;
        if !requirements.insert(grant.requirement.as_str()) {
            return Err(deployment_error(
                "deployment_grant_duplicate",
                format!(
                    "deployment requirement '{}' is granted twice",
                    grant.requirement
                ),
            ));
        }
        validate_adapter_descriptor(&grant.adapter)?;
    }
    Ok(())
}

fn validate_adapter_descriptor(adapter: &AdapterDescriptor) -> Result<(), Diagnostic> {
    match adapter {
        AdapterDescriptor::SecretVerifier { secret, .. }
        | AdapterDescriptor::Postgres {
            connection_secret: secret,
            ..
        }
        | AdapterDescriptor::DurableQueuePostgres {
            connection_secret: secret,
            ..
        } => validate_name(secret, "secret name")?,
        AdapterDescriptor::ObjectLocal { root, .. } => {
            validate_relative(root, "object root")?;
        }
        AdapterDescriptor::ObjectS3 {
            access_key_secret,
            secret_key_secret,
            ..
        } => {
            validate_name(access_key_secret, "access-key secret name")?;
            validate_name(secret_key_secret, "secret-key secret name")?;
        }
        AdapterDescriptor::Configuration
        | AdapterDescriptor::WallClock
        | AdapterDescriptor::SecureRandom
        | AdapterDescriptor::Identifier
        | AdapterDescriptor::PasswordHash { .. }
        | AdapterDescriptor::ByteStream
        | AdapterDescriptor::ObjectMemory { .. }
        | AdapterDescriptor::DurableQueueMemory { .. } => {}
    }
    Ok(())
}

fn validate_program_descriptor(
    descriptor: &DeploymentDescriptor,
    program: &NormalizedProgram,
) -> Result<(), Diagnostic> {
    let target_name = Name::new(descriptor.target.clone())?;
    let target = program.root_target(&target_name).ok_or_else(|| {
        deployment_error(
            "deployment_target_missing",
            "deployment names no exact root-package artifact target",
        )
    })?;
    validate_runner_descriptor(descriptor, target.runner)?;
    let component = program
        .components
        .get(target.component.0 as usize)
        .ok_or_else(|| {
            deployment_error(
                "deployment_component_missing",
                "selected target component escaped the exact artifact table",
            )
        })?;
    let supplied = descriptor
        .grants
        .iter()
        .map(|grant| (grant.requirement.as_str(), grant))
        .collect::<BTreeMap<_, _>>();
    for requirement_index in component.requirements.iter().copied() {
        let requirement = program
            .requirements
            .get(requirement_index.0 as usize)
            .ok_or_else(|| {
                deployment_error(
                    "deployment_requirement_missing",
                    "component requirement escaped the exact artifact table",
                )
            })?;
        let grant = supplied.get(requirement.name.as_str()).ok_or_else(|| {
            deployment_error(
                "deployment_grant_missing",
                format!(
                    "component requirement '{}' has no deployment grant",
                    requirement.name
                ),
            )
        })?;
        validate_exact_adapter_interface(requirement.interface, &grant.adapter)?;
    }
    if supplied.len() != component.requirements.len() {
        let required = component
            .requirements
            .iter()
            .filter_map(|index| program.requirements.get(index.0 as usize))
            .map(|requirement| requirement.name.as_str())
            .collect::<BTreeSet<_>>();
        let foreign = supplied
            .keys()
            .find(|alias| !required.contains(**alias))
            .copied()
            .unwrap_or("<unknown>");
        return Err(deployment_error(
            "deployment_grant_foreign",
            format!("deployment grants undeclared component requirement '{foreign}'"),
        ));
    }
    Ok(())
}

fn validate_exact_adapter_interface(
    interface: super::kernel::DeclarationReference,
    adapter: &AdapterDescriptor,
) -> Result<(), Diagnostic> {
    const STANDARD_PACKAGE: &str = "pkg_10000000000000000000000000000001";
    let declaration = match adapter {
        AdapterDescriptor::Configuration => "decl_def8eec5eed34e86eda0df7ee7bb4883",
        AdapterDescriptor::WallClock => "decl_8d99ab2f1d59391e1e21c17cc8757731",
        AdapterDescriptor::SecureRandom => "decl_2ad39598d2945149fff8b841fe8b253e",
        AdapterDescriptor::Identifier => "decl_92bb73b52bc3654abcbde47513873f42",
        AdapterDescriptor::PasswordHash { .. } => "decl_375bc0a9f5214e8a27ede17a14e79f67",
        AdapterDescriptor::SecretVerifier { .. } => "decl_172ae7f44000b32243d75a92e6733e50",
        AdapterDescriptor::ByteStream => "decl_e29e0ac407696662f355e9056172ac2b",
        AdapterDescriptor::Postgres { .. } => "decl_4c1cf20949507973e07ece4ec002c2d7",
        AdapterDescriptor::ObjectMemory { .. }
        | AdapterDescriptor::ObjectLocal { .. }
        | AdapterDescriptor::ObjectS3 { .. } => "decl_ac421d578f44958595e92fa9f5fb1d43",
        AdapterDescriptor::DurableQueueMemory { .. }
        | AdapterDescriptor::DurableQueuePostgres { .. } => "decl_20a0ef729beda0abf0e743cd7e1126de",
    };
    if interface.package.to_string() != STANDARD_PACKAGE
        || interface.declaration.to_string() != declaration
    {
        return Err(deployment_error(
            "deployment_adapter_interface",
            format!(
                "{} adapter requires its exact maintained standard interface",
                adapter.kind()
            ),
        ));
    }
    Ok(())
}

fn validate_runner_descriptor(
    descriptor: &DeploymentDescriptor,
    runner: RunnerKind,
) -> Result<(), Diagnostic> {
    match runner {
        RunnerKind::Http => {
            if descriptor.listen.is_none() || descriptor.http.is_none() {
                return Err(deployment_error(
                    "deployment_http_incomplete",
                    "HTTP target requires listen and http descriptors",
                ));
            }
            if descriptor.worker.is_some() {
                return Err(deployment_error(
                    "deployment_runner_foreign",
                    "HTTP target may not declare worker topology",
                ));
            }
        }
        RunnerKind::Worker => {
            if descriptor.worker.is_none() {
                return Err(deployment_error(
                    "deployment_worker_incomplete",
                    "worker target requires a worker descriptor",
                ));
            }
            if descriptor.listen.is_some() || descriptor.http.is_some() {
                return Err(deployment_error(
                    "deployment_runner_foreign",
                    "worker target may not declare listener or HTTP topology",
                ));
            }
        }
        RunnerKind::Command | RunnerKind::Interactive | RunnerKind::Batch | RunnerKind::Test => {
            if descriptor.listen.is_some()
                || descriptor.http.is_some()
                || descriptor.worker.is_some()
            {
                return Err(deployment_error(
                    "deployment_runner_foreign",
                    "nonresident target may not declare HTTP or worker topology",
                ));
            }
        }
    }
    Ok(())
}

fn normalized_adapter(
    adapter: &AdapterDescriptor,
    configuration: &BTreeMap<String, ConfigurationValue>,
) -> NormalizedAdapterDescriptor {
    match adapter {
        AdapterDescriptor::Configuration => NormalizedAdapterDescriptor::Configuration {
            values: configuration.clone(),
        },
        AdapterDescriptor::WallClock => NormalizedAdapterDescriptor::WallClock,
        AdapterDescriptor::SecureRandom => NormalizedAdapterDescriptor::SecureRandom,
        AdapterDescriptor::Identifier => NormalizedAdapterDescriptor::Identifier,
        AdapterDescriptor::PasswordHash { policy } => NormalizedAdapterDescriptor::PasswordHash {
            policy: policy.clone(),
        },
        AdapterDescriptor::SecretVerifier {
            secret,
            maximum_candidate_bytes,
        } => NormalizedAdapterDescriptor::SecretVerifier {
            secret: secret.clone(),
            maximum_candidate_bytes: *maximum_candidate_bytes,
        },
        AdapterDescriptor::ByteStream => NormalizedAdapterDescriptor::ByteStream,
        AdapterDescriptor::Postgres {
            connection_secret,
            maximum_connections,
            maximum_wait_milliseconds,
            statement_timeout_milliseconds,
        } => NormalizedAdapterDescriptor::Postgres {
            connection_secret: connection_secret.clone(),
            maximum_connections: *maximum_connections,
            maximum_wait_milliseconds: *maximum_wait_milliseconds,
            statement_timeout_milliseconds: *statement_timeout_milliseconds,
        },
        AdapterDescriptor::ObjectMemory { prefix, limits } => {
            NormalizedAdapterDescriptor::ObjectMemory {
                prefix: prefix.clone(),
                limits: limits.clone(),
            }
        }
        AdapterDescriptor::ObjectLocal {
            root,
            prefix,
            limits,
        } => NormalizedAdapterDescriptor::ObjectLocal {
            root: root.clone(),
            prefix: prefix.clone(),
            limits: limits.clone(),
        },
        AdapterDescriptor::ObjectS3 {
            endpoint,
            region,
            bucket,
            prefix,
            allow_http,
            path_style,
            access_key_secret,
            secret_key_secret,
            limits,
        } => NormalizedAdapterDescriptor::ObjectS3 {
            endpoint: endpoint.clone(),
            region: region.clone(),
            bucket: bucket.clone(),
            prefix: prefix.clone(),
            allow_http: *allow_http,
            path_style: *path_style,
            access_key_secret: access_key_secret.clone(),
            secret_key_secret: secret_key_secret.clone(),
            limits: limits.clone(),
        },
        AdapterDescriptor::DurableQueueMemory { limits } => {
            NormalizedAdapterDescriptor::DurableQueueMemory {
                limits: limits.clone(),
            }
        }
        AdapterDescriptor::DurableQueuePostgres {
            connection_secret,
            namespace,
            maximum_connections,
            maximum_wait_milliseconds,
            statement_timeout_milliseconds,
            limits,
        } => NormalizedAdapterDescriptor::DurableQueuePostgres {
            connection_secret: connection_secret.clone(),
            namespace: namespace.clone(),
            maximum_connections: *maximum_connections,
            maximum_wait_milliseconds: *maximum_wait_milliseconds,
            statement_timeout_milliseconds: *statement_timeout_milliseconds,
            limits: limits.clone(),
        },
    }
}

fn normalized_run_policy(policy: RunPolicy) -> NormalizedRunPolicy {
    NormalizedRunPolicy {
        instruction_steps: policy.instruction_fuel,
        maximum_call_depth: policy.maximum_call_depth,
        maximum_value_stack: policy.maximum_value_stack,
        ..NormalizedRunPolicy::default()
    }
}

fn resolve_relative(root: &Path, value: &str, label: &str) -> Result<PathBuf, Diagnostic> {
    validate_relative(value, label)?;
    let mut resolved = root.to_path_buf();
    let mut components = Path::new(value).components().peekable();
    while let Some(Component::Normal(component)) = components.next() {
        resolved.push(component);
        match fs::symlink_metadata(&resolved) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(deployment_error(
                    "deployment_input_kind",
                    format!("{label} path contains a symbolic-link component"),
                ));
            }
            Ok(metadata) if components.peek().is_some() && !metadata.is_dir() => {
                return Err(deployment_error(
                    "deployment_input_kind",
                    format!("{label} path contains a non-directory parent component"),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(root.join(value));
            }
            Err(error) => return Err(deployment_io("deployment_read", &resolved, error)),
        }
    }
    Ok(resolved)
}

fn validate_relative(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.is_empty() || value.len() > 4096 || value.contains('\0') || value.contains('\\') {
        return Err(deployment_error(
            "deployment_path",
            format!("{label} path is empty, excessive, or noncanonical"),
        ));
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(deployment_error(
            "deployment_path",
            format!("{label} path is not a canonical relative path"),
        ));
    }
    Ok(())
}

fn validate_name(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(deployment_error(
            "deployment_name",
            format!("{label} is not a canonical bounded name"),
        ));
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(deployment_error(
            "deployment_digest",
            format!("{label} must be 64 lowercase hexadecimal characters"),
        ));
    }
    Ok(())
}

fn read_bounded(path: &Path, maximum: u64, label: &str) -> Result<Vec<u8>, Diagnostic> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| deployment_io("deployment_read", path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(deployment_error(
            "deployment_input_kind",
            format!(
                "{label} '{}' is not a regular non-symlink file",
                path.display()
            ),
        ));
    }
    if metadata.len() > maximum {
        return Err(deployment_error(
            "deployment_input_limit",
            format!("{label} '{}' exceeds {maximum} bytes", path.display()),
        ));
    }
    let bytes = fs::read(path).map_err(|error| deployment_io("deployment_read", path, error))?;
    if u64::try_from(bytes.len()).map_or(true, |length| length > maximum) {
        return Err(deployment_error(
            "deployment_input_limit",
            format!("{label} '{}' exceeds {maximum} bytes", path.display()),
        ));
    }
    Ok(bytes)
}

fn deployment_io(code: &str, path: &Path, error: std::io::Error) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Infrastructure,
        code,
        format!("{}: {error}", path.display()),
    )
}

fn deployment_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "contract_version": 1,
            "artifact": "application.lkja",
            "target": "serve",
            "listen": "127.0.0.1:0",
            "runtime": ResidentLimits::default(),
            "execution": RunPolicy::default(),
            "http": HttpLimits::default(),
            "worker": null,
            "streams": StreamLimits::default(),
            "configuration": {},
            "secrets": [],
            "grants": []
        }))
        .expect("deployment JSON")
    }

    #[test]
    fn strict_current_descriptor_and_relative_paths_are_enforced() {
        assert!(decode_deployment(&minimal()).is_ok());
        let mut value: serde_json::Value =
            serde_json::from_slice(&minimal()).expect("deployment value");
        value["contract_version"] = serde_json::json!(0);
        let error =
            decode_deployment(&serde_json::to_vec(&value).expect("predecessor deployment JSON"))
                .expect_err("predecessor must reject");
        assert_eq!(error.code, "deployment_contract");
        value["contract_version"] = serde_json::json!(1);
        value["artifact"] = serde_json::json!("../foreign.lkja");
        let error = decode_deployment(&serde_json::to_vec(&value).expect("path JSON"))
            .expect_err("traversal must reject");
        assert_eq!(error.code, "deployment_path");
        value["artifact"] = serde_json::json!("foreign\\artifact.lkja");
        assert_eq!(
            decode_deployment(&serde_json::to_vec(&value).expect("backslash path JSON"))
                .expect_err("backslash must reject")
                .code,
            "deployment_path"
        );

        for path in ["", "/foreign.lkja", ".", "nested/../foreign.lkja"] {
            value["artifact"] = serde_json::json!(path);
            assert_eq!(
                decode_deployment(&serde_json::to_vec(&value).expect("path JSON"))
                    .expect_err("noncanonical path must reject")
                    .code,
                "deployment_path",
                "{path}"
            );
        }
    }

    #[test]
    fn starter_http_descriptor_is_strict_loopback_only_and_fresh() {
        let first = starter_http_deployment().expect("first starter deployment");
        let second = starter_http_deployment().expect("second starter deployment");
        assert_eq!(first.contract_version, DEPLOYMENT_CONTRACT_VERSION);
        assert_eq!(first.artifact, STARTER_HTTP_ARTIFACT_PATH);
        assert_eq!(first.target, STARTER_HTTP_TARGET);
        assert_eq!(first.listen.as_deref(), Some(STARTER_HTTP_LISTENER));
        assert_eq!(first.runtime.maximum_concurrent_tasks, 16);
        assert_eq!(first.runtime.maximum_queued_tasks, 64);
        assert_eq!(first.runtime.request_deadline_milliseconds, 30_000);
        assert_eq!(first.runtime.shutdown_grace_milliseconds, 30_000);
        assert_eq!(first.runtime.cancellation_grace_milliseconds, 5_000);
        assert_eq!(first.execution.instruction_fuel, 10_000_000);
        assert_eq!(first.execution.maximum_call_depth, 4_096);
        assert_eq!(first.execution.maximum_value_stack, 1_000_000);
        let http = first.http.as_ref().expect("HTTP limits");
        assert_eq!(http.maximum_request_body_bytes, 8 * 1024 * 1024);
        assert_eq!(http.maximum_response_body_bytes, 4 * 1024 * 1024);
        assert_eq!(http.maximum_header_bytes, 32 * 1024);
        assert_eq!(http.maximum_headers, 128);
        assert_eq!(first.streams.maximum_chunk_bytes, 64 * 1024);
        assert_eq!(first.streams.maximum_buffered_chunks, 8);
        assert_eq!(first.streams.maximum_total_bytes, 64 * 1024 * 1024);
        assert_eq!(first.streams.maximum_live_streams, 1_024);
        assert!(first.worker.is_none());
        assert!(first.configuration.is_empty());
        assert!(first.secrets.is_empty());
        assert_eq!(first.grants.len(), 1);
        let grant = &first.grants[0];
        assert_eq!(grant.requirement, "streams");
        assert_eq!(grant.sharing_domain, "http-request-streams");
        assert_eq!(grant.authority_revision.len(), 64);
        assert_ne!(grant.authority_revision, "0".repeat(64));
        assert!(matches!(grant.adapter, AdapterDescriptor::ByteStream));
        assert_ne!(
            grant.authority_revision, second.grants[0].authority_revision,
            "starter deployment authority must be freshly generated"
        );

        let bytes = encode_deployment(&first).expect("encode starter deployment");
        assert_eq!(bytes.last(), Some(&b'\n'));
        let decoded = decode_deployment(&bytes).expect("strictly decode encoded starter");
        assert_eq!(decoded.artifact, first.artifact);
        assert_eq!(decoded.target, first.target);
        assert_eq!(decoded.listen, first.listen);
        assert_eq!(
            decoded.grants[0].authority_revision,
            grant.authority_revision
        );
    }

    #[cfg(unix)]
    #[test]
    fn deployment_inputs_reject_symbolic_links_and_nonregular_files() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("deployment input directory");
        let artifact = temporary.path().join("application.lkja");
        fs::write(&artifact, b"artifact").expect("artifact fixture");
        let linked_artifact = temporary.path().join("linked.lkja");
        symlink(&artifact, &linked_artifact).expect("artifact symlink");
        assert_eq!(
            read_bounded(&linked_artifact, 64, "component artifact")
                .expect_err("artifact symlink must reject")
                .code,
            "deployment_input_kind"
        );

        let linked_directory = temporary.path().join("linked-directory");
        symlink(temporary.path(), &linked_directory).expect("directory symlink");
        assert_eq!(
            resolve_relative(
                temporary.path(),
                "linked-directory/application.lkja",
                "artifact"
            )
            .expect_err("parent symlink must reject")
            .code,
            "deployment_input_kind"
        );
        assert_eq!(
            read_bounded(temporary.path(), 64, "component artifact")
                .expect_err("directory input must reject")
                .code,
            "deployment_input_kind"
        );
        assert_eq!(
            read_bounded(&artifact, 1, "component artifact")
                .expect_err("oversized input must reject")
                .code,
            "deployment_input_limit"
        );
    }
}
