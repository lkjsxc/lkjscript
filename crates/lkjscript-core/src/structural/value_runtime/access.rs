use crate::Value;

use super::super::StructuralValueKey;
use super::{
    DestinationCleanupReport, SemanticValue, StaticStructuralArtifact, StructuralDestinationKey,
    StructuralImage, StructuralNode, StructuralObject, StructuralProjection, StructuralType,
    StructuralValueError, StructuralValueRuntime, StructuralViewKey, ViewSlot,
};

impl StructuralValueRuntime {
    pub fn value(
        &self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<SemanticValue, StructuralValueError> {
        let image = self.value_image(key, expected)?;
        image.to_semantic()
    }

    pub fn value_node(
        &self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<StructuralNode<'_>, StructuralValueError> {
        Ok(self.value_image(key, expected)?.root())
    }

    fn value_image(
        &self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<&StructuralImage, StructuralValueError> {
        let root = self
            .roots
            .root(key, expected.layout, expected.semantic_type)?;
        match self.objects.get(root)? {
            StructuralObject::Owned { image, .. } => {
                self.require_type(image.root().value_type(), expected)?;
                Ok(image)
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

    pub fn projected(&self, key: StructuralViewKey) -> Result<SemanticValue, StructuralValueError> {
        let record = self.view(key)?;
        let StructuralObject::Owned { image, .. } = self.objects.get(record.root)? else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        image.to_semantic_at(record.node)
    }

    pub fn projected_node(
        &self,
        key: StructuralViewKey,
    ) -> Result<StructuralNode<'_>, StructuralValueError> {
        let record = self.view(key)?;
        let StructuralObject::Owned { image, .. } = self.objects.get(record.root)? else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        image
            .node(record.node)
            .ok_or(StructuralValueError::InvariantViolation)
    }

    pub fn utf8_view(&self, key: StructuralViewKey) -> Result<&str, StructuralValueError> {
        let record = self.view(key)?;
        let StructuralProjection::Utf8 { start, end, .. } = record.projection else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        let StructuralObject::Owned { image, .. } = self.objects.get(record.root)? else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        let bytes = image.bytes(record.node, super::StructuralKind::String)?;
        let text = std::str::from_utf8(bytes).map_err(|_| StructuralValueError::InvalidUtf8)?;
        text.get(start as usize..end as usize)
            .ok_or(StructuralValueError::InvalidRange)
    }

    pub fn byte_vector_mut(
        &mut self,
        key: StructuralViewKey,
    ) -> Result<&mut [u8], StructuralValueError> {
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
        let root = record.root;
        let node = record.node;
        let StructuralObject::Owned { image, .. } = self.objects.get_mut(root)? else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        image.bytes_mut(node, super::StructuralKind::ByteVector)
    }

    pub fn path_equals(
        &self,
        left: StructuralValueKey,
        right: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<bool, StructuralValueError> {
        let left = self.value_image(left, expected)?;
        let right = self.value_image(right, expected)?;
        Ok(
            left.bytes(super::LocalNodeId::ROOT, super::StructuralKind::Path)?
                == right.bytes(super::LocalNodeId::ROOT, super::StructuralKind::Path)?,
        )
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
        let (image, facts) = if let Some(root) = value.as_structural_root() {
            self.preflight_root_field(key, field, root, expected)?;
            self.take_owned_image(root, expected)?
        } else {
            self.image_from_value(value, expected)?
        };
        self.initialize_image(key, field, image, facts)
            .map_err(|failure| failure.0)
    }
}
