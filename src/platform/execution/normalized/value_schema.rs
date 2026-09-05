//! Neutral value-layout access for strict boundary codecs. No executable units or resolution.

use super::prepare::{NormalizedProgram, NormalizedRecordLayout, NormalizedVariantLayout};
use crate::platform::kernel::{TypeObject, TypeObjectDigest};
use std::collections::BTreeMap;

pub trait NormalizedValueSchema {
    fn records(&self) -> &[NormalizedRecordLayout];
    fn variants(&self) -> &[NormalizedVariantLayout];
    fn types(&self) -> &BTreeMap<TypeObjectDigest, TypeObject>;
}

impl NormalizedValueSchema for NormalizedProgram {
    fn records(&self) -> &[NormalizedRecordLayout] {
        &self.records
    }
    fn variants(&self) -> &[NormalizedVariantLayout] {
        &self.variants
    }
    fn types(&self) -> &BTreeMap<TypeObjectDigest, TypeObject> {
        &self.types
    }
}
