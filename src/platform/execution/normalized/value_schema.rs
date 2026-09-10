//! Neutral value-layout access for strict boundary codecs. No executable units or resolution.

use super::prepare::{NormalizedProgram, NormalizedRecordLayout, NormalizedVariantLayout};
use crate::platform::kernel::{TypeObject, TypeObjectDigest};
use std::collections::BTreeMap;

pub trait NormalizedValueSchema {
    fn value_origin(&self) -> super::value::ValueOrigin;
    fn records(&self) -> &[NormalizedRecordLayout];
    fn variants(&self) -> &[NormalizedVariantLayout];
    fn types(&self) -> &BTreeMap<TypeObjectDigest, TypeObject>;
    fn record_index(&self, ty: TypeObjectDigest) -> Option<usize>;
    fn variant_index(&self, ty: TypeObjectDigest) -> Option<usize>;
    fn comparable(&self, ty: TypeObjectDigest) -> bool;
    fn application_free(&self, ty: TypeObjectDigest) -> bool;
}

impl NormalizedValueSchema for NormalizedProgram {
    fn comparable(&self, ty: TypeObjectDigest) -> bool {
        self.comparable_types.contains(&ty)
    }
    fn application_free(&self, ty: TypeObjectDigest) -> bool {
        self.application_free_types.contains(&ty)
    }
    fn value_origin(&self) -> super::value::ValueOrigin {
        self.value_origin
    }
    fn records(&self) -> &[NormalizedRecordLayout] {
        &self.records
    }
    fn variants(&self) -> &[NormalizedVariantLayout] {
        &self.variants
    }
    fn types(&self) -> &BTreeMap<TypeObjectDigest, TypeObject> {
        &self.types
    }
    fn record_index(&self, ty: TypeObjectDigest) -> Option<usize> {
        self.record_instances.get(&ty).map(|index| index.0 as usize)
    }
    fn variant_index(&self, ty: TypeObjectDigest) -> Option<usize> {
        self.variant_instances
            .get(&ty)
            .map(|index| index.0 as usize)
    }
}

/// Neutral canonical identity only: this derives no layout, retention proof, or origin.
pub(super) fn nominal_identity(
    declaration: crate::platform::kernel::DeclarationReference,
    arguments: &[TypeObjectDigest],
) -> Result<TypeObjectDigest, crate::platform::diagnostic::Diagnostic> {
    let form = if arguments.is_empty() {
        crate::platform::kernel::TypeForm::Named { declaration }
    } else {
        crate::platform::kernel::TypeForm::Applied {
            declaration,
            arguments: arguments.to_vec(),
        }
    };
    crate::platform::kernel::encode_type_object(&TypeObject::new(form)?).map(|(digest, _)| digest)
}
