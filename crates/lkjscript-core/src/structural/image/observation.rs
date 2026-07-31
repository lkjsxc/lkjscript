use super::super::value_runtime::{
    SemanticPayload, SemanticValue, StructuralKind, StructuralValueError,
};
use super::{discard_semantic, LocalNodeId, StructuralImage, StructuralNodePayload};

impl StructuralImage {
    pub fn to_semantic(&self) -> Result<SemanticValue, StructuralValueError> {
        self.to_semantic_at(LocalNodeId::ROOT)
    }

    pub fn to_semantic_at(&self, root: LocalNodeId) -> Result<SemanticValue, StructuralValueError> {
        self.record(root)?;
        let mut reachable = flags(self.nodes.len())?;
        let mut pending = Vec::new();
        let mut built = Vec::new();
        let mut discard = Vec::new();
        pending.try_reserve_exact(self.nodes.len())?;
        built.try_reserve_exact(self.nodes.len())?;
        discard.try_reserve_exact(self.nodes.len())?;
        built.resize_with(self.nodes.len(), || None);
        pending.push(root);
        while let Some(id) = pending.pop() {
            if std::mem::replace(&mut reachable[id.get() as usize], true) {
                continue;
            }
            match &self.record(id)?.payload {
                StructuralNodePayload::Product(range) => pending.extend(
                    Self::range(&self.fields, *range)
                        .ok_or(StructuralValueError::InvariantViolation)?,
                ),
                StructuralNodePayload::Enum { fields, .. } => pending.extend(
                    Self::range(&self.fields, *fields)
                        .ok_or(StructuralValueError::InvariantViolation)?,
                ),
                StructuralNodePayload::Inline(_)
                | StructuralNodePayload::Static(_)
                | StructuralNodePayload::Bytes(_) => {}
            }
        }
        for index in (0..self.nodes.len()).rev() {
            if !reachable[index] {
                continue;
            }
            let record = &self.nodes[index];
            let payload = match build_payload(self, record, &mut built) {
                Ok(payload) => payload,
                Err(error) => {
                    discard_built(&mut built, &mut discard);
                    return Err(error);
                }
            };
            built[index] = Some(SemanticValue::new(record.value_type, payload));
        }
        built[root.get() as usize]
            .take()
            .ok_or(StructuralValueError::InvariantViolation)
    }
}

fn build_payload(
    image: &StructuralImage,
    record: &super::StructuralNodeRecord,
    built: &mut [Option<SemanticValue>],
) -> Result<SemanticPayload, StructuralValueError> {
    match &record.payload {
        StructuralNodePayload::Inline(value) => Ok(SemanticPayload::Inline(*value)),
        StructuralNodePayload::Static(value) => Ok(SemanticPayload::Static(*value)),
        StructuralNodePayload::Bytes(range) => {
            let bytes = StructuralImage::range(&image.blob, *range)
                .ok_or(StructuralValueError::InvariantViolation)?;
            let mut copy = Vec::new();
            copy.try_reserve_exact(bytes.len())?;
            copy.extend_from_slice(bytes);
            match record.value_type.kind {
                StructuralKind::String => Ok(SemanticPayload::String(copy)),
                StructuralKind::Path => Ok(SemanticPayload::Path(copy)),
                StructuralKind::Bytes => Ok(SemanticPayload::Bytes(copy)),
                StructuralKind::ByteVector => Ok(SemanticPayload::ByteVector(copy)),
                _ => Err(StructuralValueError::InvariantViolation),
            }
        }
        StructuralNodePayload::Product(range) => Ok(SemanticPayload::Product(
            take_fields(image, *range, built)?.into(),
        )),
        StructuralNodePayload::Enum { tag, fields } => Ok(SemanticPayload::Enum {
            tag: *tag,
            active_payload: take_fields(image, *fields, built)?.into(),
        }),
    }
}

fn take_fields(
    image: &StructuralImage,
    range: super::CheckedU32Range,
    built: &mut [Option<SemanticValue>],
) -> Result<Vec<SemanticValue>, StructuralValueError> {
    let ids = StructuralImage::range(&image.fields, range)
        .ok_or(StructuralValueError::InvariantViolation)?;
    let mut fields = Vec::new();
    fields.try_reserve_exact(ids.len())?;
    for id in ids {
        fields.push(
            built[id.get() as usize]
                .take()
                .ok_or(StructuralValueError::InvariantViolation)?,
        );
    }
    Ok(fields)
}

fn discard_built(built: &mut [Option<SemanticValue>], discard: &mut Vec<SemanticValue>) {
    for value in built.iter_mut().filter_map(Option::take) {
        discard_semantic(value, discard);
    }
}

fn flags(length: usize) -> Result<Vec<bool>, StructuralValueError> {
    let mut values = Vec::new();
    values.try_reserve_exact(length)?;
    values.resize(length, false);
    Ok(values)
}
