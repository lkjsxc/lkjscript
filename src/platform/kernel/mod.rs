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
mod name;
mod owner;
mod reference;
mod relation;
mod root;
mod type_object;
mod validate;

pub use codec::{
    decode_dependency, decode_owner, decode_retirement, decode_root, decode_type_object,
    encode_dependency, encode_owner, encode_retirement, encode_root, encode_type_object,
};
pub use digest::{
    BlobObjectDigest, ChangeDigest, DependencyObjectDigest, OwnerObjectDigest, PackageObjectDigest,
    RetirementObjectDigest, SemanticRootDigest, SequenceObjectDigest, TypeObjectDigest,
};
pub use expression::*;
pub use id::{EncodedOwnerKey, ExactOwnerKey, OwnerHeader, OwnerKey, OwnerKind, PackageId};
pub use name::Name;
pub use owner::*;
pub use reference::*;
pub use relation::{RelationEdge, RelationEndpoint, RelationKind, extract_relations};
pub use root::*;
pub use type_object::*;
pub use validate::{FullValidationReport, KernelSnapshot, validate_full};

#[cfg(test)]
mod tests;
