//! Closed validation-witness contracts and feature ownership.

use super::ValidatorContractDigest;
use crate::platform::kernel::{NamespaceClass, OwnerKind, RelationKind};

pub const WITNESS_CONTRACT_IDENTITY: &str = "lkjscript-validation-witness-9";
pub const WITNESS_CONTRACT_VERSION: u16 = 9;
pub const OWNER_SUMMARY_CONTRACT_IDENTITY: &str = "lkjscript-owner-summary-9";
pub const OWNER_SUMMARY_CONTRACT_VERSION: u16 = 9;
pub const VALIDATOR_CONTRACT_IDENTITY: &str = "lkjscript-semantic-validator-14";

pub const WITNESS_MAGIC: [u8; 8] = *b"LKJWIT09";
pub const OWNER_SUMMARY_MAGIC: [u8; 8] = *b"LKJSUM14";
pub const WITNESS_ENVELOPE_DOMAIN: &str = "lkjscript.witness.envelope.v9";
pub const OWNER_SUMMARY_ENVELOPE_DOMAIN: &str = "lkjscript.owner-summary.envelope.v9";

pub const VALIDATION_WITNESS_DIGEST_DOMAIN: &str = "lkjscript.validation-witness.v8";
pub const OWNER_SUMMARY_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.v8";
pub const VALIDATION_CERTIFICATE_DIGEST_DOMAIN: &str = "lkjscript.validation-certificate.v8";
pub const VALIDATOR_CONTRACT_DIGEST_DOMAIN: &str = "lkjscript.validator-contract.v13";

pub const INTERFACE_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.interface.v8";
pub const IMPLEMENTATION_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.implementation.v7";
pub const TYPE_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.type.v6";
pub const EFFECT_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.effect.v6";
pub const CAPABILITY_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.capability.v6";
pub const RELATION_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.relation.v6";
pub const PRESENTATION_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.presentation.v6";
pub const TEST_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.test.v6";
pub const VALIDATION_DEPENDENCY_DIGEST_DOMAIN: &str =
    "lkjscript.owner-summary.validation-dependency.v6";

pub const MAXIMUM_OWNER_SUMMARY_BYTES: usize = 64 * 1024;
pub const MAXIMUM_WITNESS_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAXIMUM_OWNERSHIP_VALUE_BYTES: usize = 256;
pub const MAXIMUM_RELATION_PREFIX_ITEMS: usize = 10_000;
pub const MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS: usize = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatorFeatureDescriptor {
    pub name: &'static str,
    pub version: u16,
}

/// This list is the executable owner for rules that affect acceptance or safe witness reuse.
/// Changing one rule requires changing its feature version, which changes the validator digest.
pub const VALIDATOR_FEATURES: [ValidatorFeatureDescriptor; 22] = [
    ValidatorFeatureDescriptor {
        name: "parametric_nominal_data",
        version: 2,
    },
    ValidatorFeatureDescriptor {
        name: "capture_safe_type_parameter_constraints",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "explicit_pure_function_binding",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "graph_5_full_validation",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "canonical_namespace",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "exact_ownership",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "single_relation_extractor",
        version: 2,
    },
    ValidatorFeatureDescriptor {
        name: "owner_summary_dimensions",
        version: 2,
    },
    ValidatorFeatureDescriptor {
        name: "declaration_local_aggregation",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "validation_dependency_projection",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "test_dependency_projection",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "canonical_persistent_map_v2",
        version: 2,
    },
    ValidatorFeatureDescriptor {
        name: "full_witness_rebuild",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "generic_owner_delta",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "reverse_impact_planner",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "incremental_owner_frontier",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "path_copy_witness_update",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "exact_dependency_interfaces",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "qualified_task_requirements",
        version: 1,
    },
    ValidatorFeatureDescriptor {
        name: "affine_capability_resources",
        version: 2,
    },
    ValidatorFeatureDescriptor {
        name: "structured_session_relations",
        version: 2,
    },
    ValidatorFeatureDescriptor {
        name: "exact_http_route_topology",
        version: 2,
    },
];

pub fn validator_contract_digest() -> ValidatorContractDigest {
    let mut bytes = Vec::new();
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
        push_text(&mut bytes, feature.name);
        bytes.extend_from_slice(&feature.version.to_le_bytes());
    }
    ValidatorContractDigest::of(&bytes)
}

fn push_text(bytes: &mut Vec<u8>, value: &str) {
    let length = value.len() as u64;
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
