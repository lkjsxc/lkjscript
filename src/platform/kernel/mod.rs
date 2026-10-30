//! Current semantic kernel behind the typed process boundary.
//!
//! These normalized records and complete oracles own current program meaning. The module remains
//! crate-private so Rust representation is not elevated into the public language contract.

#![allow(
    unused_imports,
    reason = "closed kernel exports include independent test oracles and future typed adapters"
)]

mod affine;
#[cfg(test)]
mod affine_reference;
pub(crate) mod associated_flow_canonical_reference;
#[cfg(test)]
mod associated_flow_canonical_tests;
pub(crate) mod associated_flow_reference;
#[cfg(test)]
mod associated_flow_reference_tests;
pub(crate) mod callable_flow;
mod codec;
pub mod contract;
mod digest;
mod effect;
mod expression;
mod id;
mod implementation;
mod infer;
mod interface;
pub(crate) mod memory;
#[cfg(test)]
pub(crate) mod memory_reference;
mod name;
mod namespace;
pub(crate) mod owned_associated;
#[cfg(test)]
mod owned_associated_codec_tests;
mod owned_contract;
pub(crate) mod owned_product;
pub(crate) mod owned_record;
mod owner;
pub(crate) mod parallel;
pub(crate) mod parallel_types;
#[cfg(test)]
mod parallel_types_tests;
pub(crate) mod region;
pub(crate) mod share;
pub(crate) mod transfer;
pub use owned_contract::*;
pub(crate) mod read_view;
pub use read_view::*;
mod reference;
mod relation;
mod requirement;
mod root;
mod scoped;
mod state;
#[cfg(test)]
pub(crate) mod test_work;
mod type_object;
mod validate;
mod work;
pub(crate) use work::ValidationWork;

pub(crate) use affine::validate_affine_roots_with_limits;
pub use codec::{
    DEPENDENCY_BINDING_BYTES, OWNER_BINDING_BYTES, RETIREMENT_BINDING_BYTES, decode_dependency,
    decode_dependency_binding, decode_owner, decode_owner_binding, decode_retirement,
    decode_retirement_binding, decode_root, decode_type_object, encode_dependency,
    encode_dependency_binding, encode_owner, encode_owner_binding, encode_retirement,
    encode_retirement_binding, encode_root, encode_type_object,
};
pub use digest::{
    BlobObjectDigest, ChangeDigest, DependencyObjectDigest, OwnerObjectDigest,
    PackageInterfaceDigest, PackageRevisionDigest, PackageTransportDigest, RetirementObjectDigest,
    SemanticRootDigest, SemanticStateDigest, SequenceObjectDigest, TypeObjectDigest,
};
pub use effect::*;
pub use expression::*;
pub use id::{
    EncodedOwnerKey, ExactOwnerKey, IdentityKind, OwnerHeader, OwnerKey, OwnerKind, PackageId,
};
pub use implementation::ImplementationName;
pub(crate) use infer::{
    CheckedExpressionRead, ExpressionRead, ExpressionValidationExhaustion,
    ExpressionValidationLimits, infer_function_expression_type,
    infer_function_expression_type_object, validate_expression_roots,
    validate_expression_roots_with_limits,
};
pub use interface::*;
pub use name::Name;
pub use namespace::{NamespaceClass, NamespaceEntryRef, owner_namespace};
pub use owner::*;
pub use reference::*;
pub(crate) use relation::extract_relations_with_fanout_limit;
pub use relation::{
    PropagationClass, RelationEdge, RelationEndpoint, RelationExtractionWork, RelationKind,
    extract_owner_relations, extract_owner_relations_with_limit, extract_relations,
};
pub use requirement::*;
pub use root::*;
pub(crate) use scoped::*;
pub use state::{semantic_state_digest, semantic_state_digest_from_root};
pub use type_object::*;
pub use validate::{DependencyInterface, FullValidationReport, KernelSnapshot, validate_full};
pub(crate) use validate::{validate_full_checked, validate_full_with_limit};

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
mod finite_admission_tests;
#[cfg(test)]
mod implementation_scheme_tests;
#[cfg(test)]
mod owned_contract_tests;
#[cfg(test)]
mod owned_effect_tests;
#[cfg(test)]
mod owned_sequence_tests;
#[cfg(test)]
mod parallel_tests;
#[cfg(test)]
mod transfer_adversary_tests;
#[cfg(test)]
mod transfer_contract_tests;
