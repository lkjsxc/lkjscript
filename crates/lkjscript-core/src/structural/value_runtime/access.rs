use crate::Value;

use super::super::StructuralValueKey;
use super::{
    select, select_mut, DestinationCleanupReport, InlineStructuralValue, SemanticPayload,
    SemanticValue, StaticStructuralArtifact, StaticStructuralLeaf, StructuralDestinationKey,
    StructuralObject, StructuralProjection, StructuralType, StructuralValueError,
    StructuralValueRuntime, StructuralViewKey, ViewSlot,
};

impl StructuralValueRuntime {
    pub fn value(
        &self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<&SemanticValue, StructuralValueError> {
        let root = self
            .roots
            .root(key, expected.layout, expected.semantic_type)?;
        match self.objects.get(root)? {
            StructuralObject::Owned { value, .. } => {
                self.require_type(value.value_type, expected)?;
                Ok(value)
            }
            StructuralObject::Static(_) => Err(StructuralValueError::WrongPayloadKind),
        }
    }

    pub fn static_artifact(
        &self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<StaticStructuralArtifact, StructuralValueError> {
        let root = self
            .roots
            .root(key, expected.layout, expected.semantic_type)?;
        match self.objects.get(root)? {
            StructuralObject::Static(artifact) => {
                self.require_type(artifact.value_type, expected)?;
                Ok(*artifact)
            }
            StructuralObject::Owned { .. } => Err(StructuralValueError::WrongPayloadKind),
        }
    }

    pub fn projected(
        &self,
        key: StructuralViewKey,
    ) -> Result<&SemanticValue, StructuralValueError> {
        let record = self.view(key)?;
        let StructuralObject::Owned { value, .. } = self.objects.get(record.root)? else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        select(value, record.projection.path())
    }

    pub fn projected_mut(
        &mut self,
        key: StructuralViewKey,
    ) -> Result<&mut SemanticValue, StructuralValueError> {
        let record = match self.views.get(key.slot() as usize) {
            Some(ViewSlot::Live { generation, record }) if generation.get() == key.generation() => {
                record
            }
            _ => return Err(StructuralValueError::StaleView),
        };
        if !record.loan.is_exclusive() {
            return Err(StructuralValueError::RootTable(
                super::super::StructuralRootTableError::BorrowConflict,
            ));
        }
        let StructuralObject::Owned { value, .. } = self.objects.get_mut(record.root)? else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        select_mut(value, record.projection.path())
    }

    pub fn utf8_view(&self, key: StructuralViewKey) -> Result<&str, StructuralValueError> {
        let record = self.view(key)?;
        let StructuralProjection::Utf8 { start, end, .. } = record.projection else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        let StructuralObject::Owned { value, .. } = self.objects.get(record.root)? else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        let selected = select(value, record.projection.path())?;
        let SemanticPayload::String(bytes) = &selected.payload else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        let text = std::str::from_utf8(bytes).map_err(|_| StructuralValueError::InvalidUtf8)?;
        text.get(start as usize..end as usize)
            .ok_or(StructuralValueError::InvalidRange)
    }

    pub fn byte_vector_mut(
        &mut self,
        key: StructuralViewKey,
    ) -> Result<&mut Vec<u8>, StructuralValueError> {
        match &mut self.projected_mut(key)?.payload {
            SemanticPayload::ByteVector(bytes) => Ok(bytes),
            _ => Err(StructuralValueError::WrongPayloadKind),
        }
    }

    pub fn path_equals(
        &self,
        left: StructuralValueKey,
        right: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<bool, StructuralValueError> {
        let left = self.value(left, expected)?;
        let right = self.value(right, expected)?;
        match (&left.payload, &right.payload) {
            (SemanticPayload::Path(left), SemanticPayload::Path(right)) => Ok(left == right),
            _ => Err(StructuralValueError::WrongPayloadKind),
        }
    }

    pub fn cleanup_reports(&self) -> impl ExactSizeIterator<Item = &DestinationCleanupReport> {
        self.cleanup_reports.iter()
    }

    pub fn initialize_value(
        &mut self,
        key: StructuralDestinationKey,
        field: u16,
        value: Value,
    ) -> Result<(), StructuralValueError> {
        let expected = self.expected_field(key, field)?;
        let node = if let Some(root) = value.as_structural_root() {
            self.preflight_root_field(key, field, root, expected)?;
            self.take_owned_value(root, expected)?
        } else {
            self.node_from_value(value, expected)?
        };
        self.initialize_node(key, field, node)
            .map_err(|failure| failure.error)
    }

    fn expected_field(
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

    fn node_from_value(
        &self,
        value: Value,
        expected: StructuralType,
    ) -> Result<SemanticValue, StructuralValueError> {
        let payload = if value.is_unit() {
            SemanticPayload::Inline(InlineStructuralValue::Unit)
        } else if let Some(value) = value.as_bool() {
            SemanticPayload::Inline(InlineStructuralValue::Bool(value))
        } else if let Some(value) = value.as_i64() {
            SemanticPayload::Inline(InlineStructuralValue::I64(value))
        } else if let Some(value) = value.as_f64_bits() {
            SemanticPayload::Inline(InlineStructuralValue::F64Bits(value))
        } else if let Some(value) = value.as_function() {
            SemanticPayload::Static(StaticStructuralLeaf::Function(value))
        } else if let Some(value) = value.as_symbol() {
            SemanticPayload::Static(StaticStructuralLeaf::Symbol(value))
        } else if let Some(value) = value.as_static_bytes() {
            SemanticPayload::Static(StaticStructuralLeaf::Bytes(value))
        } else {
            return Err(StructuralValueError::MixedValue);
        };
        let node = SemanticValue::new(expected, payload);
        self.validate_tree(&node)?;
        Ok(node)
    }

    pub(super) fn require_owned_root(
        &self,
        root: super::super::RootKey,
        expected: StructuralType,
    ) -> Result<(), StructuralValueError> {
        match self.objects.get(root)? {
            StructuralObject::Owned { value, .. } => self.require_type(value.value_type, expected),
            StructuralObject::Static(_) => Err(StructuralValueError::WrongPayloadKind),
        }
    }
}
