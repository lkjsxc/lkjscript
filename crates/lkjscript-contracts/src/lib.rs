#![forbid(unsafe_code)]
//! Canonical descriptors and exact content identities for Current lkjscript contracts.

mod capability;
mod digest;
mod domains;
mod encoding;
mod memory;
mod model;
mod registry;
mod resource;
mod sha256;
mod vocabulary;

pub use capability::CapabilityKind;
pub use digest::ContractDigest;
pub use domains::{
    capability_status, current_contracts, AGENT_PROTOCOL, AGENT_PROTOCOL_DIGEST, AGENT_WORK_STATE,
    AGENT_WORK_STATE_DIGEST, BYTECODE, CAPABILITY_STATUS, CAPSULE_MANIFEST,
    CAPSULE_MANIFEST_DIGEST, COMPONENT_INTERFACE, DIAGNOSTICS, DIAGNOSTICS_DIGEST, LANGUAGE,
    LANGUAGE_DIGEST, MEMORY_OBLIGATIONS, MEMORY_OBLIGATIONS_DIGEST, METRICS, METRICS_DIGEST,
    MODULE_INTERFACE, NATIVE_LAYOUT, NATIVE_LAYOUT_DIGEST, PACKAGE_LOCK, PACKAGE_MANIFEST,
    REPOSITORY_GRAPH, REPOSITORY_GRAPH_DIGEST, RESOURCE_CATEGORIES, RESOURCE_CATEGORIES_DIGEST,
    RESOURCE_PROFILES, RESOURCE_PROFILES_DIGEST, RUNTIME_CALLS, RUNTIME_CALLS_DIGEST,
    SEMANTIC_RESOURCE_PLANE, SEMANTIC_RESOURCE_PLANE_DIGEST, SEMANTIC_SOURCE,
    SEMANTIC_SOURCE_DIGEST, SOURCE, SOURCE_DIGEST, STRUCTURAL_OWNERSHIP_DOMAINS,
    STRUCTURAL_OWNERSHIP_DOMAINS_DIGEST, TYPED_HIR, VERIFIED_SSA, VERIFIED_SSA_DIGEST,
};
pub use encoding::canonical_bytes;
pub use memory::{
    memory_obligations, LegacyTracedFamily, MemoryObligation, LEGACY_TRACED_FAMILIES,
};
pub use model::{
    ContractDependency, ContractDescriptor, ContractError, ContractFact, ContractItem,
    ContractItemKind, ContractName, FactOrdering, NameIdentity,
};
pub use registry::{require_exact, ContractMismatch, ContractSet, RegisteredContract};
pub use resource::ResourceKind;
pub use sha256::sha256;
pub use vocabulary::{
    is_identifier, operation_by_id, operation_by_source_name, operation_semantics_by_id,
    removed_spelling, OperationCategory, OperationEffects, OperationIdentity, OperationOwnership,
    OperationSemanticsRecord, OperationVocabularyRecord, RemovedSpelling, RuntimeLowering,
    SemanticSourceRelationship, BUILTIN_ERROR_NAMES, BYTE_TEXT_FOUNDATION_TYPE_NAMES,
    COMPILER_TRAIT_NAMES, CONTEXTUAL_FORM_NAMES, OPERATION_COUNT, PRELUDE_TYPE_NAMES,
    PRELUDE_VARIANT_NAMES, REMOVED_SPELLINGS, RESERVED_WORDS, SIMPLE_TYPE_NAMES,
    TYPE_CONSTRUCTOR_NAMES,
};

#[cfg(test)]
mod tests;
