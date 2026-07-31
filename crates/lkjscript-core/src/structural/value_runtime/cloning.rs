use super::super::StructuralValueKey;
use super::{
    SemanticPayload, SemanticValue, StructuralEventKind, StructuralObject, StructuralType,
    StructuralValueError, StructuralValueRuntime,
};

impl StructuralValueRuntime {
    pub fn clone_owned(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<StructuralValueKey, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        let loan = self.roots.borrow_shared(key)?;
        let cloned = match self.objects.get(root) {
            Ok(StructuralObject::Owned { value, .. }) => clone_node(value),
            Ok(StructuralObject::Static(_)) => Err(StructuralValueError::WrongPayloadKind),
            Err(error) => Err(error),
        };
        self.roots.end_borrow(loan)?;
        let cloned = cloned?;
        let facts = self.validate_tree(&cloned)?;
        match self.publish_owned(cloned) {
            Ok(copy) => {
                self.metrics.clones = self.metrics.clones.saturating_add(1);
                self.metrics.clone_nodes = self
                    .metrics
                    .clone_nodes
                    .saturating_add(u64::from(facts.nodes));
                self.metrics.string_bytes_cloned = self
                    .metrics
                    .string_bytes_cloned
                    .saturating_add(facts.string_bytes);
                self.metrics.path_bytes_cloned = self
                    .metrics
                    .path_bytes_cloned
                    .saturating_add(facts.path_bytes);
                self.record(
                    StructuralEventKind::Clone,
                    key.slot(),
                    u64::from(facts.nodes),
                );
                Ok(copy)
            }
            Err(failure) => {
                self.release_tree(failure.value, facts);
                Err(failure.error)
            }
        }
    }
}

fn clone_node(value: &SemanticValue) -> Result<SemanticValue, StructuralValueError> {
    let payload = match &value.payload {
        SemanticPayload::Inline(value) => SemanticPayload::Inline(*value),
        SemanticPayload::Static(value) => SemanticPayload::Static(*value),
        SemanticPayload::String(bytes) => SemanticPayload::String(clone_bytes(bytes)?),
        SemanticPayload::Path(bytes) => SemanticPayload::Path(clone_bytes(bytes)?),
        SemanticPayload::Bytes(bytes) => SemanticPayload::Bytes(clone_bytes(bytes)?),
        SemanticPayload::ByteVector(bytes) => SemanticPayload::ByteVector(clone_bytes(bytes)?),
        SemanticPayload::Product(fields) => SemanticPayload::Product(clone_fields(fields)?),
        SemanticPayload::Enum {
            tag,
            active_payload,
        } => SemanticPayload::Enum {
            tag: *tag,
            active_payload: clone_fields(active_payload)?,
        },
    };
    Ok(SemanticValue::new(value.value_type, payload))
}

fn clone_fields(fields: &[SemanticValue]) -> Result<Vec<SemanticValue>, StructuralValueError> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(fields.len())?;
    for field in fields {
        copy.push(clone_node(field)?);
    }
    Ok(copy)
}

fn clone_bytes(bytes: &[u8]) -> Result<Vec<u8>, StructuralValueError> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}
