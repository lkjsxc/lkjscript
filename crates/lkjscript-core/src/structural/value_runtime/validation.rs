use crate::Value;

use super::super::image::semantic_facts;
use super::{
    InlineStructuralValue, SemanticValue, StaticArtifactPayload, StaticStructuralArtifact,
    StaticStructuralLeaf, StructuralDestinationKey, StructuralImage, StructuralKind,
    StructuralNodePayload, StructuralObject, StructuralType, StructuralValueError,
    StructuralValueRuntime, TreeFacts,
};

impl StructuralValueRuntime {
    pub(super) fn validate_tree(
        &self,
        value: &SemanticValue,
    ) -> Result<TreeFacts, StructuralValueError> {
        semantic_facts(value, self.limits)
    }

    pub(super) fn validate_static(
        &self,
        artifact: StaticStructuralArtifact,
    ) -> Result<(), StructuralValueError> {
        match artifact.payload {
            StaticArtifactPayload::Inline(value) => {
                let expected = match value {
                    super::InlineStructuralValue::Unit => StructuralKind::Unit,
                    super::InlineStructuralValue::Bool(_) => StructuralKind::Bool,
                    super::InlineStructuralValue::I64(_) => StructuralKind::I64,
                    super::InlineStructuralValue::F64Bits(_) => StructuralKind::F64,
                };
                self.require_kind(artifact.value_type, expected)
            }
            StaticArtifactPayload::Static(_) => {
                self.require_kind(artifact.value_type, StructuralKind::Static)
            }
            StaticArtifactPayload::String(text) => {
                self.require_kind(artifact.value_type, StructuralKind::String)?;
                self.byte_length(text.as_bytes()).map(|_| ())
            }
            StaticArtifactPayload::Path(bytes) => {
                self.require_kind(artifact.value_type, StructuralKind::Path)?;
                self.validate_path(bytes)?;
                self.byte_length(bytes).map(|_| ())
            }
            StaticArtifactPayload::Bytes(bytes) => {
                self.require_kind(artifact.value_type, StructuralKind::Bytes)?;
                self.byte_length(bytes).map(|_| ())
            }
        }
    }

    pub(super) fn expected_field(
        &self,
        key: StructuralDestinationKey,
        field: u16,
    ) -> Result<StructuralType, StructuralValueError> {
        let record = self.destination(key)?;
        let index = usize::from(field);
        let value = record
            .values
            .get(index)
            .ok_or(StructuralValueError::FieldOutOfRange)?;
        if value.is_some() {
            return Err(StructuralValueError::FieldAlreadyInitialized);
        }
        Ok(record.field_types[index])
    }

    pub(super) fn preflight_field_type(
        &self,
        key: StructuralDestinationKey,
        field: u16,
        actual: StructuralType,
    ) -> Result<(), StructuralValueError> {
        let record = self.destination(key)?;
        let index = usize::from(field);
        let expected = *record
            .field_types
            .get(index)
            .ok_or(StructuralValueError::FieldOutOfRange)?;
        if record.values[index].is_some() {
            return Err(StructuralValueError::FieldAlreadyInitialized);
        }
        self.require_type(actual, expected)
    }

    pub(super) fn image_from_value(
        &self,
        value: Value,
        expected: StructuralType,
    ) -> Result<(StructuralImage, TreeFacts), StructuralValueError> {
        let payload = if value.is_unit() {
            StructuralNodePayload::Inline(InlineStructuralValue::Unit)
        } else if let Some(value) = value.as_bool() {
            StructuralNodePayload::Inline(InlineStructuralValue::Bool(value))
        } else if let Some(value) = value.as_i64() {
            StructuralNodePayload::Inline(InlineStructuralValue::I64(value))
        } else if let Some(value) = value.as_f64_bits() {
            StructuralNodePayload::Inline(InlineStructuralValue::F64Bits(value))
        } else if let Some(value) = value.as_function() {
            StructuralNodePayload::Static(StaticStructuralLeaf::Function(value))
        } else if let Some(value) = value.as_symbol() {
            StructuralNodePayload::Static(StaticStructuralLeaf::Symbol(value))
        } else if let Some(value) = value.as_static_bytes() {
            StructuralNodePayload::Static(StaticStructuralLeaf::Bytes(value))
        } else {
            return Err(StructuralValueError::MixedValue);
        };
        StructuralImage::single(expected, payload, self.limits)
    }

    pub(super) fn require_owned_root(
        &self,
        root: super::super::RootKey,
        expected: StructuralType,
    ) -> Result<(), StructuralValueError> {
        match self.objects.get(root)? {
            StructuralObject::Owned { image, .. } => {
                self.require_type(image.root().value_type(), expected)
            }
            StructuralObject::Sealed { .. } | StructuralObject::Static(_) => {
                Err(StructuralValueError::WrongOwnership)
            }
        }
    }

    pub(super) fn require_sealed_root(
        &self,
        root: super::super::RootKey,
        expected: StructuralType,
    ) -> Result<(), StructuralValueError> {
        match self.objects.get(root)? {
            StructuralObject::Sealed { image, .. } => {
                self.require_type(image.root().value_type(), expected)
            }
            StructuralObject::Owned { .. } | StructuralObject::Static(_) => {
                Err(StructuralValueError::WrongOwnership)
            }
        }
    }

    pub(super) fn require_type(
        &self,
        actual: StructuralType,
        expected: StructuralType,
    ) -> Result<(), StructuralValueError> {
        if actual.layout != expected.layout {
            return Err(StructuralValueError::WrongLayout);
        }
        if actual.semantic_type != expected.semantic_type {
            return Err(StructuralValueError::WrongSemanticType);
        }
        self.require_kind(actual, expected.kind)
    }

    fn require_kind(
        &self,
        actual: StructuralType,
        expected: StructuralKind,
    ) -> Result<(), StructuralValueError> {
        (actual.kind == expected)
            .then_some(())
            .ok_or(StructuralValueError::WrongPayloadKind)
    }

    fn byte_length(&self, bytes: &[u8]) -> Result<u64, StructuralValueError> {
        u64::try_from(bytes.len()).map_err(|_| StructuralValueError::ArithmeticOverflow)
    }

    fn validate_path(&self, bytes: &[u8]) -> Result<(), StructuralValueError> {
        if bytes.first() != Some(&b'/') || bytes.contains(&0) {
            return Err(StructuralValueError::InvalidPath);
        }
        Ok(())
    }
}
