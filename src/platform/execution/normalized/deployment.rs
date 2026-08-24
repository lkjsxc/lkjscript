//! Exact deployment preparation for normalized Graph 5 capability adapters.

use super::byte_stream::{NormalizedByteStreamAdapter, NormalizedByteStreamOperations};
use super::capability::{
    NormalizedAdapterKind, NormalizedCapabilities, NormalizedCapabilityAdapter,
    NormalizedCapabilityGrant, NormalizedCapabilityGrantDescriptor,
    NormalizedGrantAuthorityRevision, NormalizedGrantDescriptorDigest, NormalizedGrantLimit,
    NormalizedSharingDomain,
};
use super::configuration::{NormalizedConfigurationAdapter, NormalizedConfigurationOperations};
use super::password::{NormalizedPasswordHashAdapter, NormalizedPasswordHashOperations};
use super::prepare::{NormalizedOperation, NormalizedProgram, NormalizedRequirement};
use super::resource::NormalizedResourceScope;
use super::secret::NormalizedSecretVerifierAdapter;
use super::security::NormalizedSecurityAdapter;
use super::value::NormalizedValue;
use crate::platform::configuration::ConfigurationValue;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    DeclarationReference, Name, OperationReference, PackageId, RequirementReference, ResourceUnit,
    SemanticRootDigest, TypeForm, TypeObjectDigest,
};
use crate::platform::secrets::SecretCatalog;
use crate::platform::security::PasswordHashPolicy;
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::stream::{ByteStreamProducer, StreamLimits, StreamRegistry};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const MAXIMUM_NORMALIZED_DEPLOYMENT_GRANTS: usize = 1_024;

#[derive(Clone, Debug, Eq, PartialEq)]
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
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct NormalizedDeploymentResourcePolicy {
    pub streams: StreamLimits,
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub repository: RepositoryId,
    pub package: PackageId,
    pub revision: RevisionId,
    pub semantic_root: SemanticRootDigest,
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
    pub(crate) fn prepare(
        program: &NormalizedProgram,
        target: Name,
        grants: Vec<NormalizedDeploymentGrant>,
        resources: NormalizedDeploymentResourcePolicy,
        secrets: &SecretCatalog,
    ) -> Result<Self, Diagnostic> {
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
            let adapter = prepare_adapter(program, requirement, &grant.adapter, secrets)?;
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
                repository: program.root_repository,
                package: program.root_package,
                revision: program.root_revision,
                semantic_root: program.root_semantic_root,
                target,
                component: component.declaration,
                resources,
                grants: observed,
            },
        })
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

fn prepare_adapter(
    program: &NormalizedProgram,
    requirement: &NormalizedRequirement,
    descriptor: &NormalizedAdapterDescriptor,
    secrets: &SecretCatalog,
) -> Result<Arc<dyn NormalizedCapabilityAdapter>, Diagnostic> {
    let interface = requirement.interface;
    match descriptor {
        NormalizedAdapterDescriptor::Configuration { values } => {
            let exists = require_operation(program, requirement, "exists")?;
            let text = require_operation(program, requirement, "text")?;
            let i64 = require_operation(program, requirement, "i64")?;
            let bool_operation = require_operation(program, requirement, "bool")?;
            validate_signature(
                program,
                exists,
                &[ExpectedType::StaticText],
                ExpectedType::Bool,
            )?;
            validate_signature(
                program,
                text,
                &[ExpectedType::StaticText],
                ExpectedType::Text,
            )?;
            validate_signature(program, i64, &[ExpectedType::StaticText], ExpectedType::I64)?;
            validate_signature(
                program,
                bool_operation,
                &[ExpectedType::StaticText],
                ExpectedType::Bool,
            )?;
            Ok(Arc::new(NormalizedConfigurationAdapter::new(
                interface,
                NormalizedConfigurationOperations {
                    exists: exists.reference,
                    text: text.reference,
                    i64: i64.reference,
                    bool: bool_operation.reference,
                },
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
            let hash = require_operation(program, requirement, "hash")?;
            let verify = require_operation(program, requirement, "verify")?;
            let needs_upgrade = require_operation(program, requirement, "needs-upgrade")?;
            validate_signature(program, hash, &[ExpectedType::Bytes], ExpectedType::Text)?;
            validate_signature(
                program,
                verify,
                &[ExpectedType::Bytes, ExpectedType::Text],
                ExpectedType::Bool,
            )?;
            validate_signature(
                program,
                needs_upgrade,
                &[ExpectedType::Text],
                ExpectedType::Bool,
            )?;
            Ok(Arc::new(NormalizedPasswordHashAdapter::new(
                interface,
                NormalizedPasswordHashOperations {
                    hash: hash.reference,
                    verify: verify.reference,
                    needs_upgrade: needs_upgrade.reference,
                },
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
            let read = require_operation(program, requirement, "read")?;
            let close = require_operation(program, requirement, "close")?;
            let read_all = require_operation(program, requirement, "read-all")?;
            validate_signature(
                program,
                read,
                &[ExpectedType::ByteStream],
                ExpectedType::ByteStreamRead,
            )?;
            validate_signature(
                program,
                close,
                &[ExpectedType::ByteStream],
                ExpectedType::Unit,
            )?;
            validate_signature(
                program,
                read_all,
                &[ExpectedType::ByteStream, ExpectedType::I64],
                ExpectedType::Bytes,
            )?;
            Ok(Arc::new(NormalizedByteStreamAdapter::new(
                requirement.reference,
                interface,
                NormalizedByteStreamOperations {
                    read: read.reference,
                    close: close.reference,
                    read_all: read_all.reference,
                },
            )?))
        }
    }
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
    }
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
