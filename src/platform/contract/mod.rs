//! Executable ownership of public and stored contract discovery.

mod generated;
pub(crate) mod registry;
mod schema;

pub use generated::{GeneratedDocument, generated_documents};
pub use registry::{
    AuthorityEffect, BudgetProfile, CLI_CONTRACT_VERSION, ContractAuthority, ContractDescriptor,
    ContractKey, ContractManifestEntry, ContractStability, DiagnosticDescriptor,
    ExitStatusDescriptor, LimitClass, LimitDescriptor, LimitUnit, MAXIMUM_CLI_RESPONSE_BYTES,
    MAXIMUM_TRANSACTION_REQUEST_BYTES, OperationDescriptor, OperationManifestEntry, OverridePolicy,
    PredecessorPolicy, ProjectRequirement, PublicOperation, REGISTRY_CONTRACT_IDENTITY,
    REGISTRY_CONTRACT_VERSION, RegistryManifest, RegistrySection, RegistrySnapshot, SchemaId,
    TemplateDescriptor, contract_descriptors, diagnostic_descriptors, exit_status_descriptors,
    exit_status_for, limit_descriptors, nonclaims, operation_descriptors, outcome_exit_status,
    registry_snapshot, template_descriptors,
};
pub use schema::{protocol_schema, protocol_schema_bytes, protocol_schema_digest};
