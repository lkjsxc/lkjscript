//! Current Graph 10 semantic kernel behind the typed process boundary.
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
mod codec;
pub mod contract;
mod digest;
mod expression;
mod id;
mod implementation;
mod infer;
mod interface;
mod name;
mod namespace;
mod owner;
mod reference;
mod relation;
mod root;
mod scoped;
mod state;
mod type_object;
mod validate;

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
pub use expression::*;
pub use id::{
    EncodedOwnerKey, ExactOwnerKey, IdentityKind, OwnerHeader, OwnerKey, OwnerKind, PackageId,
};
pub use implementation::ImplementationName;
pub(crate) use infer::{
    ExpressionRead, ExpressionValidationExhaustion, ExpressionValidationLimits,
    infer_function_expression_type, validate_expression_roots,
    validate_expression_roots_with_limits,
};
pub use interface::*;
pub use name::Name;
pub use namespace::{NamespaceClass, NamespaceEntryRef, owner_namespace};
pub use owner::*;
pub use reference::*;
pub use relation::{
    PropagationClass, RelationEdge, RelationEndpoint, RelationKind, extract_owner_relations,
    extract_owner_relations_with_limit, extract_relations,
};
pub use root::*;
pub(crate) use scoped::*;
pub use state::{semantic_state_digest, semantic_state_digest_from_root};
pub use type_object::*;
pub use validate::{FullValidationReport, KernelSnapshot, validate_full};

#[cfg(test)]
pub(crate) mod tests;
