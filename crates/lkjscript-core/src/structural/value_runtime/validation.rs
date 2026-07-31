use super::{
    SemanticPayload, SemanticValue, StaticArtifactPayload, StaticStructuralArtifact,
    StructuralKind, StructuralType, StructuralValueError, StructuralValueLimit,
    StructuralValueRuntime, TreeFacts,
};

impl StructuralValueRuntime {
    pub(super) fn validate_tree(
        &self,
        value: &SemanticValue,
    ) -> Result<TreeFacts, StructuralValueError> {
        self.validate_node(value, 1)
    }

    fn validate_node(
        &self,
        value: &SemanticValue,
        depth: u16,
    ) -> Result<TreeFacts, StructuralValueError> {
        if depth > self.limits.max_tree_depth {
            return Err(StructuralValueError::LimitExceeded(
                StructuralValueLimit::TreeDepth,
            ));
        }
        let mut facts = TreeFacts {
            nodes: 1,
            ..TreeFacts::default()
        };
        match &value.payload {
            SemanticPayload::Inline(inline) => {
                let kind = match inline {
                    super::InlineStructuralValue::Unit => StructuralKind::Unit,
                    super::InlineStructuralValue::Bool(_) => StructuralKind::Bool,
                    super::InlineStructuralValue::I64(_) => StructuralKind::I64,
                    super::InlineStructuralValue::F64Bits(_) => StructuralKind::F64,
                };
                self.require_kind(value.value_type, kind)?;
            }
            SemanticPayload::Static(_) => {
                self.require_kind(value.value_type, StructuralKind::Static)?
            }
            SemanticPayload::String(bytes) => {
                self.require_kind(value.value_type, StructuralKind::String)?;
                std::str::from_utf8(bytes).map_err(|_| StructuralValueError::InvalidUtf8)?;
                facts.bytes = self.byte_length(bytes)?;
                facts.string_bytes = facts.bytes;
            }
            SemanticPayload::Path(bytes) => {
                self.require_kind(value.value_type, StructuralKind::Path)?;
                self.validate_path(bytes)?;
                facts.bytes = self.byte_length(bytes)?;
                facts.path_bytes = facts.bytes;
            }
            SemanticPayload::Bytes(bytes) => {
                self.require_kind(value.value_type, StructuralKind::Bytes)?;
                facts.bytes = self.byte_length(bytes)?;
            }
            SemanticPayload::ByteVector(bytes) => {
                self.require_kind(value.value_type, StructuralKind::ByteVector)?;
                facts.bytes = self.byte_length(bytes)?;
            }
            SemanticPayload::Product(fields) => {
                self.require_kind(value.value_type, StructuralKind::Product)?;
                self.validate_fields(fields, depth, &mut facts)?;
            }
            SemanticPayload::Enum { active_payload, .. } => {
                self.require_kind(value.value_type, StructuralKind::Enum)?;
                self.validate_fields(active_payload, depth, &mut facts)?;
            }
        }
        if facts.nodes > self.limits.max_tree_nodes {
            return Err(StructuralValueError::LimitExceeded(
                StructuralValueLimit::TreeNodes,
            ));
        }
        if facts.bytes > self.limits.max_payload_bytes {
            return Err(StructuralValueError::LimitExceeded(
                StructuralValueLimit::PayloadBytes,
            ));
        }
        Ok(facts)
    }

    fn validate_fields(
        &self,
        fields: &[SemanticValue],
        depth: u16,
        facts: &mut TreeFacts,
    ) -> Result<(), StructuralValueError> {
        if fields.len() > usize::from(self.limits.max_fields) {
            return Err(StructuralValueError::LimitExceeded(
                StructuralValueLimit::Fields,
            ));
        }
        for field in fields {
            *facts = facts
                .checked_add(self.validate_node(field, depth + 1)?)
                .ok_or(StructuralValueError::ArithmeticOverflow)?;
        }
        Ok(())
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
