use super::super::value_runtime::{
    StructuralFieldPath, StructuralKind, StructuralType, StructuralValueError,
    StructuralValueRuntimeLimits,
};
use super::{
    LocalNodeId, StructuralImage, StructuralNode, StructuralNodePayload, StructuralNodeRecord,
    TreeFacts,
};

impl StructuralImage {
    pub(crate) fn single(
        value_type: StructuralType,
        payload: StructuralNodePayload,
        limits: StructuralValueRuntimeLimits,
    ) -> Result<(Self, TreeFacts), StructuralValueError> {
        let facts = TreeFacts {
            nodes: 1,
            ..TreeFacts::default()
        };
        let mut nodes = Vec::new();
        nodes.try_reserve_exact(1)?;
        nodes.push(StructuralNodeRecord {
            value_type,
            payload,
        });
        let image = Self {
            nodes,
            fields: Vec::new(),
            blob: Vec::new(),
        };
        image.validate(limits, facts)?;
        Ok((image, facts))
    }

    pub(crate) fn select(
        &self,
        path: &StructuralFieldPath,
    ) -> Result<LocalNodeId, StructuralValueError> {
        let mut id = LocalNodeId::ROOT;
        for &field in path.as_slice() {
            let record = self.record(id)?;
            let range = match record.payload {
                StructuralNodePayload::Product(range) => range,
                StructuralNodePayload::Enum { fields, .. } => fields,
                _ => return Err(StructuralValueError::InvalidFieldPath),
            };
            id = *Self::range(&self.fields, range)
                .and_then(|fields| fields.get(field))
                .ok_or(StructuralValueError::InvalidFieldPath)?;
        }
        Ok(id)
    }

    pub(crate) fn selected_node(
        &self,
        path: &StructuralFieldPath,
    ) -> Result<StructuralNode<'_>, StructuralValueError> {
        let id = self.select(path)?;
        self.node(id)
            .ok_or(StructuralValueError::InvariantViolation)
    }

    pub(crate) fn bytes(
        &self,
        id: LocalNodeId,
        kind: StructuralKind,
    ) -> Result<&[u8], StructuralValueError> {
        let record = self.record(id)?;
        if record.value_type.kind != kind {
            return Err(StructuralValueError::WrongPayloadKind);
        }
        let StructuralNodePayload::Bytes(range) = record.payload else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        Self::range(&self.blob, range).ok_or(StructuralValueError::InvariantViolation)
    }

    pub(crate) fn bytes_mut(
        &mut self,
        id: LocalNodeId,
        kind: StructuralKind,
    ) -> Result<&mut [u8], StructuralValueError> {
        let record = self.record(id)?;
        if record.value_type.kind != kind {
            return Err(StructuralValueError::WrongPayloadKind);
        }
        let StructuralNodePayload::Bytes(range) = record.payload else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        let start =
            usize::try_from(range.start()).map_err(|_| StructuralValueError::InvariantViolation)?;
        let end =
            usize::try_from(range.end()).map_err(|_| StructuralValueError::InvariantViolation)?;
        self.blob
            .get_mut(start..end)
            .ok_or(StructuralValueError::InvariantViolation)
    }
}
