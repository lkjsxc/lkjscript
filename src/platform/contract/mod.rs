//! Executable ownership of public and stored contract discovery.

mod generated;
pub(crate) mod registry;

pub use generated::{GeneratedDocument, generated_documents};
pub use registry::{
    AuthorityEffect, BudgetProfile, CLI_CONTRACT_VERSION, CapabilitiesSnapshot, ContractAuthority,
    ContractDescriptor, ContractKey, ContractStability, ControlModel, DiagnosticDescriptor,
    ExitStatusDescriptor, LimitClass, LimitDescriptor, LimitUnit, MAXIMUM_CLI_RESPONSE_BYTES,
    MAXIMUM_CLI_RESPONSE_RECORDS, MAXIMUM_TRANSACTION_REQUEST_BYTES, OperationDescriptor,
    OverridePolicy, PredecessorPolicy, ProjectRequirement, PublicOperation,
    REGISTRY_CONTRACT_IDENTITY, REGISTRY_CONTRACT_VERSION, RegistrySection,
    RegistrySectionSnapshot, RegistrySnapshot, capabilities_snapshot, contract_descriptors,
    diagnostic_class_name, diagnostic_descriptors, exit_status_descriptors, exit_status_for,
    limit_descriptors, nonclaims, operation_descriptors, operation_record, outcome_exit_status,
    registry_snapshot,
};
