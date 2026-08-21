//! Strict deployment descriptors bind source-authored requirements to generic native adapters.

use super::artifact::{MAXIMUM_ARTIFACT_BYTES, load_artifact};
use super::configuration::{ConfigurationAdapter, ConfigurationObservation, ConfigurationValue};
use super::database::{PostgresAdapter, PostgresPool, PostgresPoolConfig, PostgresSecret};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{
    CAPABILITY_GRANT_CONTRACT_VERSION, CapabilityAdapter, CapabilityGrant,
    CapabilityGrantDescriptor, PreparedProgram, PreparedRequirement, RunPolicy,
};
use super::http::{HttpApplication, HttpLimits};
use super::object::{
    ObjectLimits, ObjectStorageAdapter, S3Config as ObjectS3Config, S3Credentials,
};
use super::queue::{DurableQueueAdapter, QueueLimits};
use super::runtime::{ResidentDeployment, ResidentLimits};
use super::secrets::{EnvironmentSecretBinding, SecretCatalog, SecretVerifierAdapter};
use super::security::{
    IdentifierAdapter, PasswordHashAdapter, PasswordHashPolicy, SecureRandomAdapter,
    WallClockAdapter,
};
use super::semantic::{OwnerId, ResolvedType};
use super::stream::{ByteStreamAdapter, StreamLimits, StreamRegistry};
use super::worker::{WorkerApplication, WorkerLimits};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::runtime::Handle;

pub const DEPLOYMENT_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_DEPLOYMENT_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_DEPLOYMENT_GRANTS: usize = 1_024;

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
    program: Arc<PreparedProgram>,
    grants: Vec<CapabilityGrant>,
    streams: StreamRegistry,
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
        let descriptor_bytes = read_bounded(path, MAXIMUM_DEPLOYMENT_BYTES, "deployment")?;
        let descriptor = decode_deployment(&descriptor_bytes)?;
        let directory = path.parent().unwrap_or_else(|| Path::new("."));
        let artifact_path = resolve_relative(directory, &descriptor.artifact, "artifact")?;
        let artifact_bytes =
            read_bounded(&artifact_path, MAXIMUM_ARTIFACT_BYTES, "component artifact")?;
        let program = Arc::new(PreparedProgram::prepare(load_artifact(&artifact_bytes)?)?);
        let secrets = SecretCatalog::from_environment(&descriptor.secrets)?;
        Self::prepare(descriptor, program, directory, runtime, secrets)
    }

    pub fn prepare(
        descriptor: DeploymentDescriptor,
        program: Arc<PreparedProgram>,
        deployment_directory: &Path,
        runtime: Handle,
        secrets: SecretCatalog,
    ) -> Result<Self, Diagnostic> {
        validate_descriptor(&descriptor)?;
        let target = program.target(&descriptor.target)?.clone();
        validate_runner_descriptor(&descriptor, target.runner)?;
        let component = program.components().get(&target.component).ok_or_else(|| {
            deployment_error(
                "deployment_component_missing",
                "prepared target component disappeared",
            )
        })?;
        let streams = StreamRegistry::new(descriptor.streams.clone())?;
        let configuration = ConfigurationAdapter::new(
            component
                .requirements
                .values()
                .find(|requirement| {
                    descriptor.grants.iter().any(|grant| {
                        grant.requirement == requirement.alias
                            && matches!(grant.adapter, AdapterDescriptor::Configuration)
                    })
                })
                .map(|requirement| requirement.interface.clone())
                .unwrap_or_else(|| OwnerId {
                    package: program.artifact().root_package_id.clone(),
                    module: "deployment".to_owned(),
                    declaration: "UnusedConfiguration".to_owned(),
                }),
            descriptor.configuration.clone(),
        )?;
        let configuration_observation = configuration.observe_redacted();

        let mut by_requirement = BTreeMap::new();
        for grant in &descriptor.grants {
            if by_requirement
                .insert(grant.requirement.as_str(), grant)
                .is_some()
            {
                return Err(deployment_error(
                    "deployment_grant_duplicate",
                    format!(
                        "deployment requirement '{}' is granted twice",
                        grant.requirement
                    ),
                ));
            }
        }
        if by_requirement.len() > MAXIMUM_DEPLOYMENT_GRANTS {
            return Err(deployment_error(
                "deployment_grant_limit",
                format!("deployment has more than {MAXIMUM_DEPLOYMENT_GRANTS} grants"),
            ));
        }
        let mut grants = Vec::new();
        let mut observed_grants = BTreeMap::new();
        for (alias, requirement) in &component.requirements {
            let declared = by_requirement.remove(alias.as_str()).ok_or_else(|| {
                deployment_error(
                    "deployment_grant_missing",
                    format!("component requirement '{alias}' has no deployment grant"),
                )
            })?;
            let adapter: Arc<dyn CapabilityAdapter> = match &declared.adapter {
                AdapterDescriptor::Configuration => Arc::new(ConfigurationAdapter::new(
                    requirement.interface.clone(),
                    descriptor.configuration.clone(),
                )?),
                AdapterDescriptor::WallClock => {
                    Arc::new(WallClockAdapter::new(requirement.interface.clone()))
                }
                AdapterDescriptor::SecureRandom => {
                    Arc::new(SecureRandomAdapter::new(requirement.interface.clone()))
                }
                AdapterDescriptor::Identifier => {
                    Arc::new(IdentifierAdapter::new(requirement.interface.clone()))
                }
                AdapterDescriptor::PasswordHash { policy } => Arc::new(
                    PasswordHashAdapter::new(requirement.interface.clone(), policy.clone())
                        .map_err(execution_diagnostic)?,
                ),
                AdapterDescriptor::SecretVerifier {
                    secret,
                    maximum_candidate_bytes,
                } => Arc::new(SecretVerifierAdapter::new(
                    requirement.interface.clone(),
                    secrets.require(secret)?.clone(),
                    *maximum_candidate_bytes,
                )?),
                AdapterDescriptor::ByteStream => Arc::new(ByteStreamAdapter::new(
                    requirement.interface.clone(),
                    streams.clone(),
                )),
                AdapterDescriptor::Postgres {
                    connection_secret,
                    maximum_connections,
                    maximum_wait_milliseconds,
                    statement_timeout_milliseconds,
                } => {
                    let (value_owner, type_owner) = database_nominal_owners(requirement)?;
                    let pool = postgres_pool(
                        &secrets,
                        connection_secret,
                        *maximum_connections,
                        *maximum_wait_milliseconds,
                        *statement_timeout_milliseconds,
                    )?;
                    Arc::new(PostgresAdapter::new(
                        requirement.interface.clone(),
                        value_owner,
                        type_owner,
                        pool,
                    ))
                }
                AdapterDescriptor::ObjectMemory { prefix, limits } => {
                    Arc::new(ObjectStorageAdapter::in_memory(
                        requirement.interface.clone(),
                        runtime.clone(),
                        streams.clone(),
                        prefix.clone(),
                        limits.clone(),
                    )?)
                }
                AdapterDescriptor::ObjectLocal {
                    root,
                    prefix,
                    limits,
                } => {
                    let root = resolve_relative(deployment_directory, root, "object root")?;
                    fs::create_dir_all(&root)
                        .map_err(|error| deployment_io("deployment_object_root", &root, error))?;
                    Arc::new(ObjectStorageAdapter::local(
                        requirement.interface.clone(),
                        runtime.clone(),
                        streams.clone(),
                        &root,
                        prefix.clone(),
                        limits.clone(),
                    )?)
                }
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
                } => {
                    let access_key = secrets.require(access_key_secret)?.text()?.to_owned();
                    let secret_key = secrets.require(secret_key_secret)?.text()?.to_owned();
                    Arc::new(ObjectStorageAdapter::s3(
                        requirement.interface.clone(),
                        runtime.clone(),
                        streams.clone(),
                        ObjectS3Config {
                            endpoint: endpoint.clone(),
                            region: region.clone(),
                            bucket: bucket.clone(),
                            prefix: prefix.clone(),
                            allow_http: *allow_http,
                            path_style: *path_style,
                            credentials: S3Credentials::new(access_key, secret_key)?,
                        },
                        limits.clone(),
                    )?)
                }
                AdapterDescriptor::DurableQueueMemory { limits } => Arc::new(
                    DurableQueueAdapter::in_memory(requirement.interface.clone(), limits.clone())?,
                ),
                AdapterDescriptor::DurableQueuePostgres {
                    connection_secret,
                    namespace,
                    maximum_connections,
                    maximum_wait_milliseconds,
                    statement_timeout_milliseconds,
                    limits,
                } => {
                    let pool = postgres_pool(
                        &secrets,
                        connection_secret,
                        *maximum_connections,
                        *maximum_wait_milliseconds,
                        *statement_timeout_milliseconds,
                    )?;
                    Arc::new(DurableQueueAdapter::postgres(
                        requirement.interface.clone(),
                        pool,
                        namespace.clone(),
                        limits.clone(),
                    )?)
                }
            };
            let descriptor_digest = adapter_descriptor_digest(declared)?;
            grants.push(CapabilityGrant {
                requirement: alias.clone(),
                descriptor: CapabilityGrantDescriptor {
                    contract_version: CAPABILITY_GRANT_CONTRACT_VERSION,
                    interface: requirement.interface.clone(),
                    adapter_kind: declared.adapter.kind().to_owned(),
                    sharing_domain: declared.sharing_domain.clone(),
                    authority_revision: declared.authority_revision.clone(),
                    descriptor_digest,
                    operations: requirement.operations.keys().cloned().collect(),
                    limits: requirement.limits.clone(),
                },
                adapter,
            });
            observed_grants.insert(alias.clone(), declared.adapter.kind().to_owned());
        }
        if let Some((alias, _)) = by_requirement.into_iter().next() {
            return Err(deployment_error(
                "deployment_grant_foreign",
                format!("deployment grants undeclared component requirement '{alias}'"),
            ));
        }
        let observation = DeploymentObservation {
            contract_version: DEPLOYMENT_CONTRACT_VERSION,
            artifact_digest: program.artifact().artifact_digest.clone(),
            target: descriptor.target.clone(),
            runner: format!("{:?}", target.runner).to_ascii_lowercase(),
            listen: descriptor.listen.clone(),
            configuration: configuration_observation,
            secret_names: secrets.names(),
            grants: observed_grants,
        };
        Ok(Self {
            descriptor,
            program,
            grants,
            streams,
            observation,
        })
    }

    pub fn observe_redacted(&self) -> &DeploymentObservation {
        &self.observation
    }

    pub fn listen(&self) -> Option<&str> {
        self.descriptor.listen.as_deref()
    }

    pub fn resident(&self) -> Result<ResidentDeployment, Diagnostic> {
        ResidentDeployment::prepare(
            self.program.clone(),
            &self.descriptor.target,
            self.grants.clone(),
            self.descriptor.runtime.clone(),
            self.descriptor.execution,
        )
    }

    pub fn http_application(&self) -> Result<HttpApplication, Diagnostic> {
        let limits = self.descriptor.http.clone().ok_or_else(|| {
            deployment_error(
                "deployment_http_missing",
                "HTTP target requires an HTTP limits descriptor",
            )
        })?;
        HttpApplication::new(self.resident()?, limits, self.streams.clone())
    }

    pub fn worker_application(&self) -> Result<WorkerApplication, Diagnostic> {
        let limits = self.descriptor.worker.clone().ok_or_else(|| {
            deployment_error(
                "deployment_worker_missing",
                "worker target requires a worker limits descriptor",
            )
        })?;
        WorkerApplication::new(self.resident()?, limits)
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
    for grant in &descriptor.grants {
        validate_name(&grant.requirement, "requirement")?;
        validate_name(&grant.sharing_domain, "sharing domain")?;
        validate_digest(&grant.authority_revision, "authority revision")?;
    }
    Ok(())
}

fn validate_runner_descriptor(
    descriptor: &DeploymentDescriptor,
    runner: super::package::RunnerKind,
) -> Result<(), Diagnostic> {
    use super::package::RunnerKind;
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

fn postgres_pool(
    secrets: &SecretCatalog,
    secret_name: &str,
    maximum_connections: usize,
    maximum_wait_milliseconds: u64,
    statement_timeout_milliseconds: u64,
) -> Result<PostgresPool, Diagnostic> {
    let connection = secrets.require(secret_name)?.text()?.to_owned();
    let pool = PostgresPool::new(PostgresPoolConfig {
        connection: PostgresSecret::new(connection)?,
        maximum_connections,
        maximum_wait_milliseconds,
        statement_timeout_milliseconds,
    })?;
    pool.preflight().map_err(execution_diagnostic)?;
    Ok(pool)
}

fn database_nominal_owners(
    requirement: &PreparedRequirement,
) -> Result<(OwnerId, OwnerId), Diagnostic> {
    let execute = requirement.operations.get("execute").ok_or_else(|| {
        deployment_error(
            "deployment_postgres_interface",
            "PostgreSQL adapter requires the exact execute operation",
        )
    })?;
    let query = requirement.operations.get("query").ok_or_else(|| {
        deployment_error(
            "deployment_postgres_interface",
            "PostgreSQL adapter requires the exact query operation",
        )
    })?;
    let value_owner = match execute.parameters.get(1) {
        Some(ResolvedType::List(value)) => match value.as_ref() {
            ResolvedType::Nominal(owner) => owner.clone(),
            _ => return Err(postgres_shape()),
        },
        _ => return Err(postgres_shape()),
    };
    let type_owner = match query.parameters.get(2) {
        Some(ResolvedType::List(value)) => match value.as_ref() {
            ResolvedType::Nominal(owner) => owner.clone(),
            _ => return Err(postgres_shape()),
        },
        _ => return Err(postgres_shape()),
    };
    Ok((value_owner, type_owner))
}

fn postgres_shape() -> Diagnostic {
    deployment_error(
        "deployment_postgres_interface",
        "PostgreSQL interface must use nominal SqlValue and SqlType lists in exact positions",
    )
}

fn adapter_descriptor_digest(grant: &DeploymentGrant) -> Result<String, Diagnostic> {
    let bytes = serde_json::to_vec(grant).map_err(|error| {
        deployment_error(
            "deployment_descriptor_digest",
            format!("adapter descriptor could not be encoded: {error}"),
        )
    })?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn resolve_relative(root: &Path, value: &str, label: &str) -> Result<PathBuf, Diagnostic> {
    validate_relative(value, label)?;
    Ok(root.join(value))
}

fn validate_relative(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.is_empty() || value.len() > 4096 || value.contains('\0') {
        return Err(deployment_error(
            "deployment_path",
            format!("{label} path is empty, excessive, or contains NUL"),
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

fn read_bounded(path: &Path, maximum: usize, label: &str) -> Result<Vec<u8>, Diagnostic> {
    let metadata =
        fs::metadata(path).map_err(|error| deployment_io("deployment_read", path, error))?;
    if metadata.len() > maximum as u64 {
        return Err(deployment_error(
            "deployment_input_limit",
            format!("{label} '{}' exceeds {maximum} bytes", path.display()),
        ));
    }
    let bytes = fs::read(path).map_err(|error| deployment_io("deployment_read", path, error))?;
    if bytes.len() > maximum {
        return Err(deployment_error(
            "deployment_input_limit",
            format!("{label} '{}' exceeds {maximum} bytes", path.display()),
        ));
    }
    Ok(bytes)
}

fn execution_diagnostic(error: super::execution::ExecutionError) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Capability, error.code, error.message)
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
    }
}
