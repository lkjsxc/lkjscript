#![forbid(unsafe_code)]
//! Canonical descriptors and exact content identities for Current lkjscript contracts.

mod digest;
mod domains;
mod encoding;
mod model;
mod registry;
mod sha256;

pub use digest::ContractDigest;
pub use domains::{
    capability_status, current_contracts, AGENT_PROTOCOL, AGENT_PROTOCOL_DIGEST, AGENT_WORK_STATE,
    AGENT_WORK_STATE_DIGEST, BYTECODE, CAPABILITY_STATUS, CAPSULE_MANIFEST,
    CAPSULE_MANIFEST_DIGEST, COMPONENT_INTERFACE, DIAGNOSTICS, DIAGNOSTICS_DIGEST, LANGUAGE,
    LANGUAGE_DIGEST, METRICS, METRICS_DIGEST, MODULE_INTERFACE, NATIVE_LAYOUT,
    NATIVE_LAYOUT_DIGEST, PACKAGE_LOCK, PACKAGE_MANIFEST, REPOSITORY_GRAPH,
    REPOSITORY_GRAPH_DIGEST, RESOURCE_CATEGORIES, RESOURCE_CATEGORIES_DIGEST, RESOURCE_PROFILES,
    RESOURCE_PROFILES_DIGEST, RUNTIME_CALLS, RUNTIME_CALLS_DIGEST, SEMANTIC_SOURCE,
    SEMANTIC_SOURCE_DIGEST, SOURCE, SOURCE_DIGEST, TYPED_HIR, VERIFIED_SSA, VERIFIED_SSA_DIGEST,
};
pub use encoding::canonical_bytes;
pub use model::{
    ContractDependency, ContractDescriptor, ContractError, ContractFact, ContractItem,
    ContractItemKind, ContractName, FactOrdering, NameIdentity,
};
pub use registry::{require_exact, ContractMismatch, ContractSet, RegisteredContract};
pub use sha256::sha256;

#[cfg(test)]
mod tests;
