//! Private Graph Contract 5 semantic kernel under construction.
//!
//! This module is deliberately not wired to repository opening or public commands yet. It owns
//! the normalized records and independent oracles that must be complete before the direct public
//! cutover. Graph Contract 4 remains the accepted public authority until that cutover.

#![allow(
    unused_imports,
    reason = "private Graph 5 exports become crate consumers at the direct cutover"
)]

mod codec;
pub mod contract;
mod digest;
mod expression;
mod id;
mod infer;
mod interface;
mod name;
mod namespace;
mod owner;
mod reference;
mod relation;
mod root;
mod state;
mod type_object;
mod validate;

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
pub use id::{EncodedOwnerKey, ExactOwnerKey, OwnerHeader, OwnerKey, OwnerKind, PackageId};
pub(crate) use infer::{ExpressionRead, validate_expression_roots};
pub use interface::*;
pub use name::Name;
pub use namespace::{NamespaceClass, NamespaceEntryRef, owner_namespace};
pub use owner::*;
pub use reference::*;
pub use relation::{
    PropagationClass, RelationEdge, RelationEndpoint, RelationKind, extract_owner_relations,
    extract_relations,
};
pub use root::*;
pub use state::{semantic_state_digest, semantic_state_digest_from_root};
pub use type_object::*;
pub use validate::{FullValidationReport, KernelSnapshot, validate_full};

#[cfg(test)]
pub(crate) mod tests;
