//! Closed validation-witness contracts and feature ownership.

use super::ValidatorContractDigest;
use crate::platform::kernel::{NamespaceClass, OwnerKind, RelationKind};

pub const WITNESS_CONTRACT_IDENTITY: &str = "lkjscript-validation-witness";
pub const WITNESS_CONTRACT_VERSION: u16 = 10;
pub const OWNER_SUMMARY_CONTRACT_IDENTITY: &str = "lkjscript-owner-summary";
pub const OWNER_SUMMARY_CONTRACT_VERSION: u16 = 12;
pub const VALIDATOR_CONTRACT_IDENTITY: &str = "lkjscript-semantic-validator";
pub const VALIDATOR_CONTRACT_VERSION: u16 = 41;

pub const WITNESS_MAGIC: [u8; 8] = *b"LKJWITNS";
pub const OWNER_SUMMARY_MAGIC: [u8; 8] = *b"LKJSUMRY";
pub const WITNESS_ENVELOPE_DOMAIN: &str = "lkjscript.witness.envelope";
pub const OWNER_SUMMARY_ENVELOPE_DOMAIN: &str = "lkjscript.owner-summary.envelope";

pub const VALIDATION_WITNESS_DIGEST_DOMAIN: &str = "lkjscript.validation-witness";
pub const OWNER_SUMMARY_DIGEST_DOMAIN: &str = "lkjscript.owner-summary";
pub const VALIDATION_CERTIFICATE_DIGEST_DOMAIN: &str = "lkjscript.validation-certificate";
pub const VALIDATOR_CONTRACT_DIGEST_DOMAIN: &str = "lkjscript.validator-contract";

pub const INTERFACE_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.interface";
pub const IMPLEMENTATION_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.implementation";
pub const TYPE_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.type";
pub const EFFECT_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.effect";
pub const CAPABILITY_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.capability";
pub const RELATION_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.relation";
pub const PRESENTATION_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.presentation";
pub const TEST_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.test";
pub const VALIDATION_DEPENDENCY_DIGEST_DOMAIN: &str =
    "lkjscript.owner-summary.validation-dependency";

pub const MAXIMUM_OWNER_SUMMARY_BYTES: usize = 64 * 1024;
pub const MAXIMUM_WITNESS_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAXIMUM_OWNERSHIP_VALUE_BYTES: usize = 256;
pub const MAXIMUM_RELATION_PREFIX_ITEMS: usize = 10_000;
pub const MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS: usize = 10_000;

/// Semantic obligations owned by the current validator implementation.
/// Its generated implementation hash invalidates proof reuse when their code changes.
pub const VALIDATOR_FEATURES: [&str; 44] = [
    "opaque_nominal_owned_records",
    "scoped_shareable_owned_inputs",
    "source_tied_owned_read_results",
    "parameterized_owned_contracts",
    "structural_owned_sequences",
    "lexical_owned_child_borrows",
    "explicit_transferable_type_parameters",
    "structured_parallel_owned_tasks",
    "structural_owned_choices",
    "structural_owned_products",
    "symbolic_owned_parameters",
    "explicit_owned_implementation_witnesses",
    "sealed_owned_i64_cells",
    "closed_external_signatures",
    "canonical_binary64_computation",
    "lexical_transaction_outcome",
    "explicit_requirement_parameters",
    "finite_generic_callable_instantiation",
    "parametric_nominal_data",
    "capture_safe_type_parameter_constraints",
    "explicit_pure_function_binding",
    "full_graph_validation",
    "canonical_namespace",
    "exact_ownership",
    "single_relation_extractor",
    "owner_summary_dimensions",
    "declaration_local_aggregation",
    "validation_dependency_projection",
    "test_dependency_projection",
    "canonical_persistent_map",
    "full_witness_rebuild",
    "generic_owner_delta",
    "reverse_impact_planner",
    "incremental_owner_frontier",
    "path_copy_witness_update",
    "exact_dependency_interfaces",
    "qualified_task_requirements",
    "affine_capability_resources",
    "owned_byte_buffers",
    "same_task_owned_transfer",
    "structured_session_relations",
    "exact_http_route_topology",
    "witness_selected_owned_associated_types",
    "associated_source_tied_read_view_families",
];

pub fn validator_contract_digest() -> ValidatorContractDigest {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&crate::platform::canonical_schema::CANONICAL_SCHEMA_HASH);
    bytes.extend_from_slice(&crate::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH);
    push_text(&mut bytes, VALIDATOR_CONTRACT_IDENTITY);
    push_text(
        &mut bytes,
        crate::platform::kernel::contract::GRAPH_CONTRACT_IDENTITY,
    );
    bytes.extend_from_slice(
        &crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION.to_le_bytes(),
    );
    push_text(&mut bytes, WITNESS_CONTRACT_IDENTITY);
    bytes.extend_from_slice(&WITNESS_CONTRACT_VERSION.to_le_bytes());
    push_text(&mut bytes, OWNER_SUMMARY_CONTRACT_IDENTITY);
    bytes.extend_from_slice(&OWNER_SUMMARY_CONTRACT_VERSION.to_le_bytes());
    bytes.extend(OwnerKind::ALL.into_iter().map(OwnerKind::tag));
    bytes.extend(NamespaceClass::ALL.into_iter().map(NamespaceClass::tag));
    for relation in RelationKind::ALL {
        bytes.push(relation.tag());
        bytes.push(relation.propagation().tag());
    }
    for feature in VALIDATOR_FEATURES {
        push_text(&mut bytes, feature);
    }
    ValidatorContractDigest::of(&bytes)
}

fn push_text(bytes: &mut Vec<u8>, value: &str) {
    let length = value.len() as u64;
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
