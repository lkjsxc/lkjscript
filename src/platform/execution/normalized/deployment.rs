//! Exact deployment preparation for normalized Graph 5 capability adapters.

use super::byte_stream::{NormalizedByteStreamAdapter, NormalizedByteStreamOperation};
use super::capability::{
    NormalizedAdapterKind, NormalizedCapabilities, NormalizedCapabilityAdapter,
    NormalizedCapabilityGrant, NormalizedCapabilityGrantDescriptor,
    NormalizedGrantAuthorityRevision, NormalizedGrantDescriptorDigest, NormalizedGrantLimit,
    NormalizedSharingDomain,
};
use super::configuration::NormalizedConfigurationAdapter;
use super::database::NormalizedPostgresAdapter;
use super::object::NormalizedObjectStorageAdapter;
use super::password::{NormalizedPasswordHashAdapter, NormalizedPasswordHashOperation};
use super::prepare::{NormalizedOperation, NormalizedProgram, NormalizedRequirement};
use super::queue::NormalizedDurableQueueAdapter;
use super::resource::NormalizedResourceScope;
use super::secret::NormalizedSecretVerifierAdapter;
use super::security::NormalizedSecurityAdapter;
use super::value::NormalizedValue;
use crate::platform::compiler::ArtifactManifestDigest;
use crate::platform::configuration::{ConfigurationOperation, ConfigurationValue};
use crate::platform::database::{PostgresPool, PostgresPoolConfig, PostgresSecret};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    DeclarationReference, Name, OperationReference, PackageId, RequirementReference, ResourceUnit,
    SemanticStateDigest, TypeForm, TypeObjectDigest,
};
use crate::platform::object::{
    ObjectEngine, ObjectLimits, S3Config as ObjectS3Config, S3Credentials,
};
use crate::platform::queue::{DurableQueueEngine, QueueLimits};
use crate::platform::secrets::SecretCatalog;
use crate::platform::security::PasswordHashPolicy;
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::stream::{ByteStreamProducer, StreamLimits, StreamRegistry};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::runtime::Handle;

const MAXIMUM_NORMALIZED_DEPLOYMENT_GRANTS: usize = 1_024;

#[derive(Clone, Debug)]
pub(crate) enum NormalizedAdapterDescriptor {
    Configuration {
        values: BTreeMap<String, ConfigurationValue>,
    },
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

impl NormalizedAdapterDescriptor {
    const fn kind(&self) -> NormalizedAdapterKind {
        match self {
            Self::Configuration { .. } => NormalizedAdapterKind::Configuration,
            Self::WallClock => NormalizedAdapterKind::WallClock,
            Self::SecureRandom => NormalizedAdapterKind::SecureRandom,
            Self::Identifier => NormalizedAdapterKind::Identifier,
            Self::PasswordHash { .. } => NormalizedAdapterKind::PasswordHash,
            Self::SecretVerifier { .. } => NormalizedAdapterKind::SecretVerifier,
            Self::ByteStream => NormalizedAdapterKind::ByteStream,
            Self::Postgres { .. } => NormalizedAdapterKind::Postgres,
            Self::ObjectMemory { .. } => NormalizedAdapterKind::ObjectMemory,
            Self::ObjectLocal { .. } => NormalizedAdapterKind::ObjectLocal,
            Self::ObjectS3 { .. } => NormalizedAdapterKind::ObjectS3,
            Self::DurableQueueMemory { .. } => NormalizedAdapterKind::DurableQueueMemory,
            Self::DurableQueuePostgres { .. } => NormalizedAdapterKind::DurableQueuePostgres,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct NormalizedDeploymentResourcePolicy {
    pub streams: StreamLimits,
}

#[derive(Clone, Debug)]
pub(crate) struct NormalizedDeploymentGrant {
    pub requirement: RequirementReference,
    pub sharing_domain: NormalizedSharingDomain,
    pub authority_revision: NormalizedGrantAuthorityRevision,
    pub limits: BTreeMap<Name, NormalizedGrantLimit>,
    pub adapter: NormalizedAdapterDescriptor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NormalizedDeploymentGrantObservation {
    pub adapter_kind: NormalizedAdapterKind,
    pub sharing_domain: NormalizedSharingDomain,
    pub authority_revision: NormalizedGrantAuthorityRevision,
    pub descriptor_digest: NormalizedGrantDescriptorDigest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NormalizedDeploymentObservation {
    pub artifact_manifest: ArtifactManifestDigest,
    pub repository: RepositoryId,
    pub package: PackageId,
    pub revision: RevisionId,
    pub semantic_state: SemanticStateDigest,
    pub target: Name,
    pub component: DeclarationReference,
    pub resources: NormalizedDeploymentResourcePolicy,
    pub grants: BTreeMap<RequirementReference, NormalizedDeploymentGrantObservation>,
}

#[derive(Clone, Debug)]
pub(crate) struct NormalizedPreparedDeployment {
    target: Name,
    capabilities: NormalizedCapabilities,
    streams: StreamRegistry,
    observation: NormalizedDeploymentObservation,
}

impl NormalizedPreparedDeployment {
    #[cfg(test)]
    pub(crate) fn prepare_exact_for_test(
        program: &NormalizedProgram,
        target: Name,
        bindings: Vec<NormalizedCapabilityGrant>,
        resources: NormalizedDeploymentResourcePolicy,
    ) -> Result<Self, Diagnostic> {
        let prepared_adapters = bindings
            .iter()
            .map(|binding| Arc::clone(&binding.adapter))
            .collect();
        let prepared: Result<Self, Diagnostic> = (|| {
            if bindings.len() > MAXIMUM_NORMALIZED_DEPLOYMENT_GRANTS {
                return Err(deployment_error(
                    DiagnosticClass::Resource,
                    "normalized_deployment_grant_limit",
                    format!(
                        "deployment has more than {MAXIMUM_NORMALIZED_DEPLOYMENT_GRANTS} exact grants"
                    ),
                ));
            }
            let target_record = program.root_target(&target).ok_or_else(|| {
                deployment_error(
                    DiagnosticClass::Source,
                    "normalized_deployment_target",
                    "deployment names no exact root-package target",
                )
            })?;
            let component = program
                .components
                .get(target_record.component.0 as usize)
                .ok_or_else(|| {
                    deployment_error(
                        DiagnosticClass::Corrupt,
                        "normalized_deployment_component",
                        "selected target component escaped the exact artifact table",
                    )
                })?;
            let streams = StreamRegistry::new(resources.streams.clone())?;
            let mut observed = BTreeMap::new();
            for binding in &bindings {
                let descriptor = &binding.descriptor;
                if observed
                    .insert(
                        binding.requirement,
                        NormalizedDeploymentGrantObservation {
                            adapter_kind: descriptor.adapter_kind,
                            sharing_domain: descriptor.sharing_domain.clone(),
                            authority_revision: descriptor.authority_revision,
                            descriptor_digest: descriptor.descriptor_digest,
                        },
                    )
                    .is_some()
                {
                    return Err(deployment_error(
                        DiagnosticClass::Capability,
                        "normalized_deployment_grant_duplicate",
                        "deployment repeats one exact requirement grant",
                    ));
                }
            }
            let capabilities =
                NormalizedCapabilities::bind(program, target_record.component, bindings)
                    .map_err(execution_diagnostic)?;
            Ok(Self {
                target: target.clone(),
                capabilities,
                streams,
                observation: NormalizedDeploymentObservation {
                    artifact_manifest: program.artifact().manifest_digest,
                    repository: program.root_repository,
                    package: program.root_package,
                    revision: program.root_revision,
                    semantic_state: program.root_semantic_state,
                    target,
                    component: component.declaration,
                    resources,
                    grants: observed,
                },
            })
        })();
        finish_preparation(prepared, prepared_adapters)
    }

    #[cfg(test)]
    pub(crate) fn prepare(
        program: &NormalizedProgram,
        target: Name,
        grants: Vec<NormalizedDeploymentGrant>,
        resources: NormalizedDeploymentResourcePolicy,
        secrets: &SecretCatalog,
    ) -> Result<Self, Diagnostic> {
        static TEST_RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> =
            std::sync::OnceLock::new();
        let runtime = Handle::try_current().unwrap_or_else(|_| {
            TEST_RUNTIME
                .get_or_init(|| {
                    tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(1)
                        .enable_all()
                        .build()
                        .unwrap_or_else(|_| std::process::abort())
                })
                .handle()
                .clone()
        });
        Self::prepare_inner(
            program,
            target,
            grants,
            resources,
            secrets,
            Path::new("."),
            runtime,
            false,
        )
    }

    pub(crate) fn prepare_with_host(
        program: &NormalizedProgram,
        target: Name,
        grants: Vec<NormalizedDeploymentGrant>,
        resources: NormalizedDeploymentResourcePolicy,
        secrets: &SecretCatalog,
        deployment_directory: &Path,
        runtime: Handle,
    ) -> Result<Self, Diagnostic> {
        Self::prepare_inner(
            program,
            target,
            grants,
            resources,
            secrets,
            deployment_directory,
            runtime,
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_inner(
        program: &NormalizedProgram,
        target: Name,
        grants: Vec<NormalizedDeploymentGrant>,
        resources: NormalizedDeploymentResourcePolicy,
        secrets: &SecretCatalog,
        deployment_directory: &Path,
        runtime: Handle,
        enforce_standard_interfaces: bool,
    ) -> Result<Self, Diagnostic> {
        let mut prepared_adapters: Vec<Arc<dyn NormalizedCapabilityAdapter>> = Vec::new();
        let prepared: Result<Self, Diagnostic> = (|| {
            if grants.len() > MAXIMUM_NORMALIZED_DEPLOYMENT_GRANTS {
                return Err(deployment_error(
                    DiagnosticClass::Resource,
                    "normalized_deployment_grant_limit",
                    format!(
                        "deployment has more than {MAXIMUM_NORMALIZED_DEPLOYMENT_GRANTS} exact grants"
                    ),
                ));
            }
            let target_record = program.root_target(&target).ok_or_else(|| {
                deployment_error(
                    DiagnosticClass::Source,
                    "normalized_deployment_target",
                    "deployment names no exact root-package target",
                )
            })?;
            let component_index = target_record.component;
            let component = program
                .components
                .get(component_index.0 as usize)
                .ok_or_else(|| {
                    deployment_error(
                        DiagnosticClass::Corrupt,
                        "normalized_deployment_component",
                        "selected target component escaped the exact artifact table",
                    )
                })?;
            let streams = StreamRegistry::new(resources.streams.clone())?;
            let mut supplied = BTreeMap::new();
            for grant in grants {
                if supplied.insert(grant.requirement, grant).is_some() {
                    return Err(deployment_error(
                        DiagnosticClass::Capability,
                        "normalized_deployment_grant_duplicate",
                        "deployment repeats one exact requirement grant",
                    ));
                }
            }
            let stream_requirements = supplied
                .values()
                .filter(|grant| grant.adapter.kind() == NormalizedAdapterKind::ByteStream)
                .map(|grant| grant.requirement)
                .collect::<Vec<_>>();

            let mut bindings = Vec::with_capacity(component.requirements.len());
            let mut observed = BTreeMap::new();
            for requirement_index in component.requirements.iter().copied() {
                let requirement = program
                    .requirements
                    .get(requirement_index.0 as usize)
                    .ok_or_else(|| {
                        deployment_error(
                            DiagnosticClass::Corrupt,
                            "normalized_deployment_requirement",
                            "component requirement escaped the exact artifact table",
                        )
                    })?;
                let grant = supplied.remove(&requirement.reference).ok_or_else(|| {
                    deployment_error(
                        DiagnosticClass::Capability,
                        "normalized_deployment_grant_missing",
                        format!(
                            "deployment omits exact requirement '{}' ({:?})",
                            requirement.name, requirement.reference
                        ),
                    )
                })?;
                let adapter_kind = grant.adapter.kind();
                let adapter = prepare_adapter(
                    program,
                    requirement,
                    &grant.adapter,
                    secrets,
                    deployment_directory,
                    &runtime,
                    enforce_standard_interfaces,
                    &stream_requirements,
                )?;
                prepared_adapters.push(Arc::clone(&adapter));
                let required_operations = exact_operations(program, requirement)?;
                if adapter.operations() != &required_operations {
                    return Err(deployment_error(
                        DiagnosticClass::Capability,
                        "normalized_deployment_operation_set",
                        "adapter shape does not cover the exact graph requirement operation set",
                    ));
                }
                let descriptor_digest = descriptor_digest(
                    &grant,
                    requirement.interface,
                    &required_operations,
                    &resources,
                );
                let descriptor = NormalizedCapabilityGrantDescriptor {
                    interface: requirement.interface,
                    adapter_kind,
                    sharing_domain: grant.sharing_domain.clone(),
                    authority_revision: grant.authority_revision,
                    descriptor_digest,
                    operations: required_operations,
                    limits: grant.limits,
                };
                observed.insert(
                    requirement.reference,
                    NormalizedDeploymentGrantObservation {
                        adapter_kind,
                        sharing_domain: descriptor.sharing_domain.clone(),
                        authority_revision: descriptor.authority_revision,
                        descriptor_digest,
                    },
                );
                bindings.push(NormalizedCapabilityGrant {
                    requirement: requirement.reference,
                    descriptor,
                    adapter,
                });
            }
            if let Some((requirement, _)) = supplied.into_iter().next() {
                return Err(deployment_error(
                    DiagnosticClass::Capability,
                    "normalized_deployment_grant_foreign",
                    format!(
                        "deployment grants requirement {requirement:?} outside the selected component"
                    ),
                ));
            }
            let capabilities = NormalizedCapabilities::bind(program, component_index, bindings)
                .map_err(execution_diagnostic)?;
            Ok(Self {
                target: target.clone(),
                capabilities,
                streams,
                observation: NormalizedDeploymentObservation {
                    artifact_manifest: program.artifact().manifest_digest,
                    repository: program.root_repository,
                    package: program.root_package,
                    revision: program.root_revision,
                    semantic_state: program.root_semantic_state,
                    target,
                    component: component.declaration,
                    resources,
                    grants: observed,
                },
            })
        })();
        finish_preparation(prepared, prepared_adapters)
    }

    pub(crate) fn target(&self) -> &Name {
        &self.target
    }

    pub(crate) fn capabilities(&self) -> &NormalizedCapabilities {
        &self.capabilities
    }

    pub(crate) fn observation(&self) -> &NormalizedDeploymentObservation {
        &self.observation
    }

    pub(crate) fn register_memory_stream(
        &self,
        requirement: RequirementReference,
        resources: &NormalizedResourceScope,
        bytes: Vec<u8>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.require_stream_grant(requirement)?;
        let lease = self.streams.register_memory(bytes)?;
        resources
            .register_byte_stream(requirement, lease)
            .map(NormalizedValue::Resource)
    }

    pub(crate) fn register_pipe_stream(
        &self,
        requirement: RequirementReference,
        resources: &NormalizedResourceScope,
        maximum_total_bytes: u64,
    ) -> Result<(NormalizedValue, ByteStreamProducer), ExecutionError> {
        self.require_stream_grant(requirement)?;
        let (lease, producer) = self.streams.register_pipe_with_limit(maximum_total_bytes)?;
        let handle = resources.register_byte_stream(requirement, lease)?;
        Ok((NormalizedValue::Resource(handle), producer))
    }

    fn require_stream_grant(
        &self,
        requirement: RequirementReference,
    ) -> Result<(), ExecutionError> {
        if self
            .observation
            .grants
            .get(&requirement)
            .is_some_and(|grant| grant.adapter_kind == NormalizedAdapterKind::ByteStream)
        {
            Ok(())
        } else {
            Err(ExecutionError::new(
                ExecutionFailureClass::Capability,
                "normalized_stream_grant",
                "deployment has no byte-stream adapter for the exact requirement",
            ))
        }
    }

    #[cfg(test)]
    pub(crate) fn live_streams(&self) -> usize {
        self.streams.live_streams()
    }
}

fn finish_preparation(
    prepared: Result<NormalizedPreparedDeployment, Diagnostic>,
    adapters: Vec<Arc<dyn NormalizedCapabilityAdapter>>,
) -> Result<NormalizedPreparedDeployment, Diagnostic> {
    match prepared {
        Ok(deployment) => Ok(deployment),
        Err(mut error) => {
            for cleanup in adapters
                .into_iter()
                .rev()
                .filter_map(|adapter| adapter.shutdown().err())
            {
                error.notes.push(format!(
                    "adapter cleanup failed with safe code '{}'",
                    cleanup.code
                ));
            }
            Err(error)
        }
    }
}

fn prepare_adapter(
    program: &NormalizedProgram,
    requirement: &NormalizedRequirement,
    descriptor: &NormalizedAdapterDescriptor,
    secrets: &SecretCatalog,
    deployment_directory: &Path,
    runtime: &Handle,
    enforce_standard_interface: bool,
    stream_requirements: &[RequirementReference],
) -> Result<Arc<dyn NormalizedCapabilityAdapter>, Diagnostic> {
    let interface = requirement.interface;
    if enforce_standard_interface {
        require_standard_interface(interface, descriptor.kind())?;
    }
    match descriptor {
        NormalizedAdapterDescriptor::Configuration { values } => {
            let mut selected = BTreeMap::new();
            for operation in requirement_operations(program, requirement)? {
                let (kind, result) = match operation.name.as_str() {
                    "exists" => (ConfigurationOperation::Exists, ExpectedType::Bool),
                    "text" => (ConfigurationOperation::Text, ExpectedType::Text),
                    "i64" => (ConfigurationOperation::I64, ExpectedType::I64),
                    "bool" => (ConfigurationOperation::Bool, ExpectedType::Bool),
                    _ => {
                        return Err(adapter_operation(&operation.name, "configuration"));
                    }
                };
                validate_signature(program, operation, &[ExpectedType::StaticText], result)?;
                selected.insert(operation.reference, kind);
            }
            Ok(Arc::new(NormalizedConfigurationAdapter::new_selected(
                interface,
                selected,
                values.clone(),
            )?))
        }
        NormalizedAdapterDescriptor::WallClock => {
            let operation = require_operation(program, requirement, "utc-milliseconds")?;
            validate_signature(program, operation, &[], ExpectedType::I64)?;
            Ok(Arc::new(NormalizedSecurityAdapter::wall_clock(
                interface,
                operation.reference,
            )?))
        }
        NormalizedAdapterDescriptor::SecureRandom => {
            let operation = require_operation(program, requirement, "bytes")?;
            validate_signature(
                program,
                operation,
                &[ExpectedType::I64],
                ExpectedType::Bytes,
            )?;
            Ok(Arc::new(NormalizedSecurityAdapter::secure_random(
                interface,
                operation.reference,
            )?))
        }
        NormalizedAdapterDescriptor::Identifier => {
            let operation = require_operation(program, requirement, "uuid-v4")?;
            validate_signature(program, operation, &[], ExpectedType::Text)?;
            Ok(Arc::new(NormalizedSecurityAdapter::identifier(
                interface,
                operation.reference,
            )?))
        }
        NormalizedAdapterDescriptor::PasswordHash { policy } => {
            let mut selected = BTreeMap::new();
            for operation in requirement_operations(program, requirement)? {
                let kind = match operation.name.as_str() {
                    "hash" => {
                        validate_signature(
                            program,
                            operation,
                            &[ExpectedType::Bytes],
                            ExpectedType::Text,
                        )?;
                        NormalizedPasswordHashOperation::Hash
                    }
                    "verify" => {
                        validate_signature(
                            program,
                            operation,
                            &[ExpectedType::Bytes, ExpectedType::Text],
                            ExpectedType::Bool,
                        )?;
                        NormalizedPasswordHashOperation::Verify
                    }
                    "needs-upgrade" => {
                        validate_signature(
                            program,
                            operation,
                            &[ExpectedType::Text],
                            ExpectedType::Bool,
                        )?;
                        NormalizedPasswordHashOperation::NeedsUpgrade
                    }
                    _ => return Err(adapter_operation(&operation.name, "password-hash")),
                };
                selected.insert(operation.reference, kind);
            }
            Ok(Arc::new(NormalizedPasswordHashAdapter::new_selected(
                interface,
                selected,
                policy.clone(),
            )?))
        }
        NormalizedAdapterDescriptor::SecretVerifier {
            secret,
            maximum_candidate_bytes,
        } => {
            let operation = require_operation(program, requirement, "matches")?;
            validate_signature(
                program,
                operation,
                &[ExpectedType::Bytes],
                ExpectedType::Bool,
            )?;
            Ok(Arc::new(NormalizedSecretVerifierAdapter::new(
                interface,
                operation.reference,
                secrets.require(secret)?.clone(),
                *maximum_candidate_bytes,
            )?))
        }
        NormalizedAdapterDescriptor::ByteStream => {
            let mut selected = BTreeMap::new();
            for operation in requirement_operations(program, requirement)? {
                let kind = match operation.name.as_str() {
                    "read" => {
                        validate_signature(
                            program,
                            operation,
                            &[ExpectedType::ByteStream],
                            ExpectedType::ByteStreamRead,
                        )?;
                        NormalizedByteStreamOperation::Read
                    }
                    "close" => {
                        validate_signature(
                            program,
                            operation,
                            &[ExpectedType::ByteStream],
                            ExpectedType::Unit,
                        )?;
                        NormalizedByteStreamOperation::Close
                    }
                    "read-all" => {
                        validate_signature(
                            program,
                            operation,
                            &[ExpectedType::ByteStream, ExpectedType::I64],
                            ExpectedType::Bytes,
                        )?;
                        NormalizedByteStreamOperation::ReadAll
                    }
                    _ => return Err(adapter_operation(&operation.name, "byte-stream")),
                };
                selected.insert(operation.reference, kind);
            }
            Ok(Arc::new(NormalizedByteStreamAdapter::new_selected(
                requirement.reference,
                interface,
                selected,
            )?))
        }
        NormalizedAdapterDescriptor::Postgres {
            connection_secret,
            maximum_connections,
            maximum_wait_milliseconds,
            statement_timeout_milliseconds,
        } => {
            let pool = postgres_pool(
                secrets,
                connection_secret,
                *maximum_connections,
                *maximum_wait_milliseconds,
                *statement_timeout_milliseconds,
            )?;
            let adapter = NormalizedPostgresAdapter::prepare(program, requirement, pool)?;
            finish_preflight(&adapter, adapter.preflight())?;
            Ok(Arc::new(adapter))
        }
        NormalizedAdapterDescriptor::ObjectMemory { prefix, limits } => {
            let engine = ObjectEngine::in_memory(runtime.clone(), prefix.clone(), limits.clone())?;
            Ok(Arc::new(NormalizedObjectStorageAdapter::prepare(
                program,
                requirement,
                NormalizedAdapterKind::ObjectMemory,
                stream_requirements,
                engine,
            )?))
        }
        NormalizedAdapterDescriptor::ObjectLocal {
            root,
            prefix,
            limits,
        } => {
            let root = resolve_relative_directory(deployment_directory, root, "object root")?;
            let engine =
                ObjectEngine::local(runtime.clone(), &root, prefix.clone(), limits.clone())?;
            Ok(Arc::new(NormalizedObjectStorageAdapter::prepare(
                program,
                requirement,
                NormalizedAdapterKind::ObjectLocal,
                stream_requirements,
                engine,
            )?))
        }
        NormalizedAdapterDescriptor::ObjectS3 {
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
            let engine = ObjectEngine::s3(
                runtime.clone(),
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
            )?;
            Ok(Arc::new(NormalizedObjectStorageAdapter::prepare(
                program,
                requirement,
                NormalizedAdapterKind::ObjectS3,
                stream_requirements,
                engine,
            )?))
        }
        NormalizedAdapterDescriptor::DurableQueueMemory { limits } => {
            let engine = DurableQueueEngine::in_memory(limits.clone())?;
            let adapter = NormalizedDurableQueueAdapter::prepare(
                program,
                requirement,
                NormalizedAdapterKind::DurableQueueMemory,
                engine,
            )?;
            finish_preflight(&adapter, adapter.preflight())?;
            Ok(Arc::new(adapter))
        }
        NormalizedAdapterDescriptor::DurableQueuePostgres {
            connection_secret,
            namespace,
            maximum_connections,
            maximum_wait_milliseconds,
            statement_timeout_milliseconds,
            limits,
        } => {
            let pool = postgres_pool(
                secrets,
                connection_secret,
                *maximum_connections,
                *maximum_wait_milliseconds,
                *statement_timeout_milliseconds,
            )?;
            let engine = DurableQueueEngine::postgres(pool, namespace.clone(), limits.clone())?;
            let adapter = NormalizedDurableQueueAdapter::prepare(
                program,
                requirement,
                NormalizedAdapterKind::DurableQueuePostgres,
                engine,
            )?;
            finish_preflight(&adapter, adapter.preflight())?;
            Ok(Arc::new(adapter))
        }
    }
}

fn finish_preflight(
    adapter: &dyn NormalizedCapabilityAdapter,
    preflight: Result<(), ExecutionError>,
) -> Result<(), Diagnostic> {
    match preflight {
        Ok(()) => Ok(()),
        Err(error) => {
            let mut diagnostic = execution_diagnostic(error);
            if let Err(cleanup) = adapter.shutdown() {
                diagnostic.notes.push(format!(
                    "adapter cleanup failed with safe code '{}'",
                    cleanup.code
                ));
            }
            Err(diagnostic)
        }
    }
}

fn requirement_operations<'a>(
    program: &'a NormalizedProgram,
    requirement: &NormalizedRequirement,
) -> Result<Vec<&'a NormalizedOperation>, Diagnostic> {
    requirement
        .operations
        .iter()
        .map(|index| {
            program.operations.get(index.0 as usize).ok_or_else(|| {
                deployment_error(
                    DiagnosticClass::Corrupt,
                    "normalized_deployment_operation",
                    "requirement operation escaped the exact artifact table",
                )
            })
        })
        .collect()
}

fn adapter_operation(name: &Name, adapter: &str) -> Diagnostic {
    deployment_error(
        DiagnosticClass::Capability,
        "normalized_deployment_adapter_operation",
        format!("{adapter} adapter does not implement exact operation '{name}'"),
    )
}

fn require_standard_interface(
    interface: DeclarationReference,
    kind: NormalizedAdapterKind,
) -> Result<(), Diagnostic> {
    const STANDARD_PACKAGE: &str = "pkg_10000000000000000000000000000001";
    let declaration = match kind {
        NormalizedAdapterKind::Configuration => "decl_def8eec5eed34e86eda0df7ee7bb4883",
        NormalizedAdapterKind::WallClock => "decl_8d99ab2f1d59391e1e21c17cc8757731",
        NormalizedAdapterKind::SecureRandom => "decl_2ad39598d2945149fff8b841fe8b253e",
        NormalizedAdapterKind::Identifier => "decl_92bb73b52bc3654abcbde47513873f42",
        NormalizedAdapterKind::PasswordHash => "decl_375bc0a9f5214e8a27ede17a14e79f67",
        NormalizedAdapterKind::SecretVerifier => "decl_172ae7f44000b32243d75a92e6733e50",
        NormalizedAdapterKind::ByteStream => "decl_e29e0ac407696662f355e9056172ac2b",
        NormalizedAdapterKind::Postgres => "decl_4c1cf20949507973e07ece4ec002c2d7",
        NormalizedAdapterKind::ObjectMemory
        | NormalizedAdapterKind::ObjectLocal
        | NormalizedAdapterKind::ObjectS3 => "decl_ac421d578f44958595e92fa9f5fb1d43",
        NormalizedAdapterKind::DurableQueueMemory | NormalizedAdapterKind::DurableQueuePostgres => {
            "decl_20a0ef729beda0abf0e743cd7e1126de"
        }
    };
    if interface.package.to_string() != STANDARD_PACKAGE
        || interface.declaration.to_string() != declaration
    {
        return Err(deployment_error(
            DiagnosticClass::Capability,
            "normalized_deployment_adapter_interface",
            format!(
                "{} adapter requires its exact maintained standard interface",
                kind.as_str()
            ),
        ));
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
    PostgresPool::new(PostgresPoolConfig {
        connection: PostgresSecret::new(connection)?,
        maximum_connections,
        maximum_wait_milliseconds,
        statement_timeout_milliseconds,
    })
}

fn resolve_relative_directory(
    root: &Path,
    value: &str,
    label: &str,
) -> Result<PathBuf, Diagnostic> {
    if value.is_empty() || value.len() > 4096 || value.contains('\0') || value.contains('\\') {
        return Err(deployment_error(
            DiagnosticClass::Source,
            "normalized_deployment_path",
            format!("{label} path is empty, excessive, or noncanonical"),
        ));
    }
    let relative = Path::new(value);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(deployment_error(
            DiagnosticClass::Source,
            "normalized_deployment_path",
            format!("{label} path is not a canonical relative path"),
        ));
    }
    let mut path = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Err(deployment_error(
                DiagnosticClass::Source,
                "normalized_deployment_path",
                format!("{label} path is not a canonical relative path"),
            ));
        };
        path.push(component);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(deployment_error(
                    DiagnosticClass::Source,
                    "normalized_deployment_directory_kind",
                    format!(
                        "{label} must contain only real directories rather than symlinks or special files"
                    ),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(deployment_error(
                    DiagnosticClass::Source,
                    "normalized_deployment_directory_missing",
                    format!("{label} must name an existing host directory"),
                ));
            }
            Err(error) => {
                return Err(deployment_error(
                    DiagnosticClass::Infrastructure,
                    "normalized_deployment_directory_read",
                    format!("{label} could not be inspected: {error}"),
                ));
            }
        }
    }
    Ok(path)
}

fn exact_operations(
    program: &NormalizedProgram,
    requirement: &NormalizedRequirement,
) -> Result<BTreeSet<OperationReference>, Diagnostic> {
    requirement
        .operations
        .iter()
        .map(|operation| {
            program
                .operations
                .get(operation.0 as usize)
                .map(|operation| operation.reference)
                .ok_or_else(|| {
                    deployment_error(
                        DiagnosticClass::Corrupt,
                        "normalized_deployment_operation",
                        "requirement operation escaped the exact artifact table",
                    )
                })
        })
        .collect()
}

fn require_operation<'a>(
    program: &'a NormalizedProgram,
    requirement: &NormalizedRequirement,
    name: &str,
) -> Result<&'a NormalizedOperation, Diagnostic> {
    let mut matches = requirement.operations.iter().filter_map(|operation| {
        program
            .operations
            .get(operation.0 as usize)
            .filter(|operation| operation.name.as_str() == name)
    });
    let operation = matches.next().ok_or_else(|| {
        deployment_error(
            DiagnosticClass::Capability,
            "normalized_deployment_adapter_operation",
            format!("adapter requires exact operation '{name}'"),
        )
    })?;
    if matches.next().is_some() {
        return Err(deployment_error(
            DiagnosticClass::Corrupt,
            "normalized_deployment_operation_name_duplicate",
            format!("requirement contains duplicate operation name '{name}'"),
        ));
    }
    Ok(operation)
}

#[derive(Clone, Copy)]
enum ExpectedType {
    Unit,
    Bool,
    I64,
    Bytes,
    Text,
    StaticText,
    ByteStream,
    ByteStreamRead,
}

fn validate_signature(
    program: &NormalizedProgram,
    operation: &NormalizedOperation,
    parameters: &[ExpectedType],
    result: ExpectedType,
) -> Result<(), Diagnostic> {
    if operation.parameters.len() != parameters.len()
        || operation
            .parameters
            .iter()
            .zip(parameters)
            .any(|(actual, expected)| !type_matches(program, actual.ty, *expected))
        || !type_matches(program, operation.result, result)
    {
        return Err(deployment_error(
            DiagnosticClass::Capability,
            "normalized_deployment_adapter_signature",
            format!(
                "exact operation '{}' has a signature incompatible with its adapter",
                operation.name
            ),
        ));
    }
    Ok(())
}

fn type_matches(
    program: &NormalizedProgram,
    digest: TypeObjectDigest,
    expected: ExpectedType,
) -> bool {
    let Some(object) = program.types.get(&digest) else {
        return false;
    };
    match (&object.form, expected) {
        (TypeForm::Unit, ExpectedType::Unit)
        | (TypeForm::Bool, ExpectedType::Bool)
        | (TypeForm::I64, ExpectedType::I64)
        | (TypeForm::Bytes, ExpectedType::Bytes)
        | (TypeForm::Text, ExpectedType::Text)
        | (TypeForm::StaticText, ExpectedType::StaticText) => true,
        (TypeForm::Stream { item }, ExpectedType::ByteStream) => {
            type_matches(program, *item, ExpectedType::Bytes)
        }
        (TypeForm::StructuralRecord { fields }, ExpectedType::ByteStreamRead) => {
            fields.len() == 2
                && fields[0].name.as_str() == "chunk"
                && type_matches(program, fields[0].ty, ExpectedType::Bytes)
                && fields[1].name.as_str() == "done"
                && type_matches(program, fields[1].ty, ExpectedType::Bool)
        }
        _ => false,
    }
}

fn descriptor_digest(
    grant: &NormalizedDeploymentGrant,
    interface: DeclarationReference,
    operations: &BTreeSet<OperationReference>,
    resources: &NormalizedDeploymentResourcePolicy,
) -> NormalizedGrantDescriptorDigest {
    let mut bytes = Vec::new();
    bytes.push(1);
    encode_requirement(&mut bytes, grant.requirement);
    encode_declaration(&mut bytes, interface);
    encode_bytes(&mut bytes, grant.adapter.kind().as_str().as_bytes());
    encode_bytes(
        &mut bytes,
        grant.sharing_domain.as_name().as_str().as_bytes(),
    );
    bytes.extend_from_slice(&grant.authority_revision.bytes());
    encode_u64(&mut bytes, operations.len() as u64);
    for operation in operations {
        encode_operation(&mut bytes, *operation);
    }
    encode_u64(&mut bytes, grant.limits.len() as u64);
    for (name, limit) in &grant.limits {
        encode_bytes(&mut bytes, name.as_str().as_bytes());
        encode_u64(&mut bytes, limit.maximum);
        bytes.push(resource_unit_tag(limit.unit));
    }
    encode_adapter(&mut bytes, &grant.adapter, resources);
    NormalizedGrantDescriptorDigest::of(&bytes)
}

fn encode_adapter(
    bytes: &mut Vec<u8>,
    adapter: &NormalizedAdapterDescriptor,
    resources: &NormalizedDeploymentResourcePolicy,
) {
    match adapter {
        NormalizedAdapterDescriptor::Configuration { values } => {
            bytes.push(1);
            encode_u64(bytes, values.len() as u64);
            for (name, value) in values {
                encode_bytes(bytes, name.as_bytes());
                match value {
                    ConfigurationValue::Text(value) => {
                        bytes.push(1);
                        encode_bytes(bytes, value.as_bytes());
                    }
                    ConfigurationValue::I64(value) => {
                        bytes.push(2);
                        bytes.extend_from_slice(&value.to_be_bytes());
                    }
                    ConfigurationValue::Bool(value) => {
                        bytes.push(3);
                        bytes.push(u8::from(*value));
                    }
                }
            }
        }
        NormalizedAdapterDescriptor::WallClock => bytes.push(2),
        NormalizedAdapterDescriptor::SecureRandom => bytes.push(3),
        NormalizedAdapterDescriptor::Identifier => bytes.push(4),
        NormalizedAdapterDescriptor::PasswordHash { policy } => {
            bytes.push(5);
            bytes.extend_from_slice(&policy.memory_kibibytes.to_be_bytes());
            bytes.extend_from_slice(&policy.iterations.to_be_bytes());
            bytes.extend_from_slice(&policy.lanes.to_be_bytes());
            encode_u64(bytes, policy.output_bytes as u64);
        }
        NormalizedAdapterDescriptor::SecretVerifier {
            secret,
            maximum_candidate_bytes,
        } => {
            bytes.push(6);
            encode_bytes(bytes, secret.as_bytes());
            encode_u64(bytes, *maximum_candidate_bytes as u64);
        }
        NormalizedAdapterDescriptor::ByteStream => {
            bytes.push(7);
            encode_u64(bytes, resources.streams.maximum_chunk_bytes as u64);
            encode_u64(bytes, resources.streams.maximum_buffered_chunks as u64);
            encode_u64(bytes, resources.streams.maximum_total_bytes);
            encode_u64(bytes, resources.streams.maximum_live_streams as u64);
        }
        NormalizedAdapterDescriptor::Postgres {
            connection_secret,
            maximum_connections,
            maximum_wait_milliseconds,
            statement_timeout_milliseconds,
        } => {
            bytes.push(8);
            encode_bytes(bytes, connection_secret.as_bytes());
            encode_u64(bytes, *maximum_connections as u64);
            encode_u64(bytes, *maximum_wait_milliseconds);
            encode_u64(bytes, *statement_timeout_milliseconds);
        }
        NormalizedAdapterDescriptor::ObjectMemory { prefix, limits } => {
            bytes.push(9);
            encode_bytes(bytes, prefix.as_bytes());
            encode_object_limits(bytes, limits);
        }
        NormalizedAdapterDescriptor::ObjectLocal {
            root,
            prefix,
            limits,
        } => {
            bytes.push(10);
            encode_bytes(bytes, root.as_bytes());
            encode_bytes(bytes, prefix.as_bytes());
            encode_object_limits(bytes, limits);
        }
        NormalizedAdapterDescriptor::ObjectS3 {
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
            bytes.push(11);
            for value in [
                endpoint,
                region,
                bucket,
                prefix,
                access_key_secret,
                secret_key_secret,
            ] {
                encode_bytes(bytes, value.as_bytes());
            }
            bytes.push(u8::from(*allow_http));
            bytes.push(u8::from(*path_style));
            encode_object_limits(bytes, limits);
        }
        NormalizedAdapterDescriptor::DurableQueueMemory { limits } => {
            bytes.push(12);
            encode_queue_limits(bytes, limits);
        }
        NormalizedAdapterDescriptor::DurableQueuePostgres {
            connection_secret,
            namespace,
            maximum_connections,
            maximum_wait_milliseconds,
            statement_timeout_milliseconds,
            limits,
        } => {
            bytes.push(13);
            encode_bytes(bytes, connection_secret.as_bytes());
            encode_bytes(bytes, namespace.as_bytes());
            encode_u64(bytes, *maximum_connections as u64);
            encode_u64(bytes, *maximum_wait_milliseconds);
            encode_u64(bytes, *statement_timeout_milliseconds);
            encode_queue_limits(bytes, limits);
        }
    }
}

fn encode_object_limits(bytes: &mut Vec<u8>, limits: &ObjectLimits) {
    encode_u64(bytes, limits.maximum_object_bytes);
    encode_u64(bytes, limits.maximum_whole_read_bytes as u64);
}

fn encode_queue_limits(bytes: &mut Vec<u8>, limits: &QueueLimits) {
    encode_u64(bytes, limits.maximum_payload_bytes as u64);
    encode_u64(bytes, limits.maximum_result_bytes as u64);
    bytes.extend_from_slice(&limits.maximum_lease_milliseconds.to_be_bytes());
    bytes.extend_from_slice(&limits.maximum_attempts.to_be_bytes());
}

fn encode_requirement(bytes: &mut Vec<u8>, reference: RequirementReference) {
    bytes.extend_from_slice(&reference.package.bytes());
    bytes.extend_from_slice(&reference.requirement.bytes());
}

fn encode_declaration(bytes: &mut Vec<u8>, reference: DeclarationReference) {
    bytes.extend_from_slice(&reference.package.bytes());
    bytes.extend_from_slice(&reference.declaration.bytes());
}

fn encode_operation(bytes: &mut Vec<u8>, reference: OperationReference) {
    bytes.extend_from_slice(&reference.package.bytes());
    bytes.extend_from_slice(&reference.operation.bytes());
}

fn encode_bytes(output: &mut Vec<u8>, bytes: &[u8]) {
    encode_u64(output, bytes.len() as u64);
    output.extend_from_slice(bytes);
}

fn encode_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_be_bytes());
}

const fn resource_unit_tag(unit: ResourceUnit) -> u8 {
    match unit {
        ResourceUnit::Bytes => 1,
        ResourceUnit::Items => 2,
        ResourceUnit::Calls => 3,
        ResourceUnit::Tasks => 4,
        ResourceUnit::Milliseconds => 5,
    }
}

fn execution_diagnostic(error: ExecutionError) -> Diagnostic {
    let class = match error.class {
        ExecutionFailureClass::Trap => DiagnosticClass::Semantic,
        ExecutionFailureClass::Capability | ExecutionFailureClass::PossibleVisibility => {
            DiagnosticClass::Capability
        }
        ExecutionFailureClass::Resource => DiagnosticClass::Resource,
        ExecutionFailureClass::Cancelled => DiagnosticClass::Cancelled,
        ExecutionFailureClass::Infrastructure => DiagnosticClass::Infrastructure,
    };
    Diagnostic::new(class, error.code, error.message)
}

fn deployment_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::resolve_relative_directory;

    #[test]
    fn local_object_roots_are_existing_canonical_host_directories() {
        let temporary = tempfile::tempdir().expect("deployment directory");
        let object_root = temporary.path().join("state/objects");
        std::fs::create_dir_all(&object_root).expect("object host directory");
        assert_eq!(
            resolve_relative_directory(temporary.path(), "state/objects", "object root")
                .expect("existing object directory"),
            object_root
        );
        assert_eq!(
            resolve_relative_directory(temporary.path(), "state/missing", "object root")
                .expect_err("missing host directory")
                .code,
            "normalized_deployment_directory_missing"
        );
        assert_eq!(
            resolve_relative_directory(temporary.path(), "../objects", "object root")
                .expect_err("traversing host directory")
                .code,
            "normalized_deployment_path"
        );
    }

    #[cfg(unix)]
    #[test]
    fn local_object_roots_reject_symbolic_link_components() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("deployment directory");
        let outside = tempfile::tempdir().expect("outside object directory");
        let state = temporary.path().join("state");
        std::fs::create_dir(&state).expect("state directory");
        symlink(outside.path(), state.join("objects")).expect("object root symlink");
        assert_eq!(
            resolve_relative_directory(temporary.path(), "state/objects", "object root")
                .expect_err("symlink host directory")
                .code,
            "normalized_deployment_directory_kind"
        );
    }
}
