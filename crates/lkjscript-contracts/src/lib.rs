#![forbid(unsafe_code)]
//! Exact identities for retained external contracts and canonical shared vocabulary.

mod capability;
mod digest;
mod domains;
#[cfg(test)]
mod encoding;
mod memory;
#[cfg(test)]
mod model;
mod resource;
mod sha256;
mod vocabulary;

pub use capability::CapabilityKind;
pub use digest::ContractDigest;
pub use domains::{
    LANGUAGE, LANGUAGE_DIGEST, MEMORY_OBLIGATIONS, MEMORY_OBLIGATIONS_DIGEST, METRICS,
    METRICS_DIGEST, MODULE_INTERFACE, MODULE_INTERFACE_DIGEST, PACKAGE_LOCK, PACKAGE_LOCK_DIGEST,
    PACKAGE_MANIFEST, PACKAGE_MANIFEST_DIGEST, SOURCE, SOURCE_DIGEST,
};
pub use memory::{
    canonical_executable_memory_witness_dependencies,
    canonical_executable_memory_witness_group_descriptor, canonical_semantic_descriptor,
    direct_nominal, executable_memory_witness_group_id, executable_memory_witness_member_id,
    memory_obligations, memory_witness_routes_are_compatible, required_memory_witness_operations,
    semantic_contract_hash, semantic_dependency_requirements, semantic_type_closure_hash,
    validate_executable_dependencies, validate_executable_memory_witness_groups,
    validate_semantic_descriptor, ExecutableMemoryWitnessDependency, ExecutableMemoryWitnessFacts,
    ExecutableMemoryWitnessGroup, ExecutableMemoryWitnessGroupError,
    ExecutableMemoryWitnessGroupMember, ExecutableMemoryWitnessRole, ExecutableMemoryWitnessTarget,
    MemoryObligation, MemoryWitnessCapabilities, MemoryWitnessContention, MemoryWitnessCopy,
    MemoryWitnessDomain, MemoryWitnessDrop, MemoryWitnessEquality, MemoryWitnessListElement,
    MemoryWitnessMode, MemoryWitnessOperation, MemoryWitnessPortability, MemoryWitnessRoot,
    MemoryWitnessSize, MemoryWitnessSnapshot, SemanticContractError, SemanticDeclaration,
    SemanticDescriptor, SemanticEnumDeclaration, SemanticEnumVariant, SemanticEnumVariantField,
    SemanticPrimitiveKind, SemanticProductDeclaration, SemanticProductField, SemanticType,
};
pub use resource::ResourceKind;
pub use sha256::{sha256, Sha256};
pub use vocabulary::{
    is_identifier, operation_by_id, operation_by_source_name, operation_semantics_by_id,
    removed_spelling, OperationCategory, OperationEffects, OperationIdentity, OperationOwnership,
    OperationSemanticsRecord, OperationVocabularyRecord, RemovedSpelling, RuntimeLowering,
    SemanticConstructor, BUILTIN_ERROR_NAMES, BYTE_TEXT_FOUNDATION_TYPE_NAMES,
    COMPILER_TRAIT_NAMES, CONTEXTUAL_FORM_NAMES, OPERATION_COUNT, PRELUDE_TYPE_NAMES,
    PRELUDE_VARIANT_NAMES, REMOVED_SPELLINGS, RESERVED_WORDS, SIMPLE_TYPE_NAMES,
    TYPE_CONSTRUCTOR_NAMES,
};

#[cfg(test)]
pub(crate) use encoding::canonical_bytes;
#[cfg(test)]
pub(crate) use model::{
    ContractDescriptor, ContractError, ContractFact, ContractItem, ContractItemKind, ContractName,
};

#[cfg(test)]
mod tests;
