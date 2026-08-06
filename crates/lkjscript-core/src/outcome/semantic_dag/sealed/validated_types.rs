use super::super::{SemanticDagKind, SemanticDagType};
use super::model::SealedSemanticDagError;
use crate::{
    StructuralFieldMetadata, StructuralFieldRoute, StructuralKind, StructuralSliceExt,
    StructuralType, StructuralTypeId, StructuralTypeMetadata, StructuralTypeMode,
    StructuralValueCategory, ValidatedChunk,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExpectedType {
    Structural(StructuralTypeId),
    Inline(StructuralType),
}

pub(super) fn return_type(chunk: &ValidatedChunk) -> Result<ExpectedType, SealedSemanticDagError> {
    let id = chunk
        .main()
        .return_structural
        .ok_or(SealedSemanticDagError::MissingValidatedReturn)?;
    let representation = chunk
        .structural_representations()
        .get_structural(id)
        .filter(|item| item.id == id && item.category == StructuralValueCategory::Owner)
        .ok_or(SealedSemanticDagError::MissingValidatedReturn)?;
    eligible_structural_type(chunk, representation.type_id)?;
    Ok(ExpectedType::Structural(representation.type_id))
}

pub(super) fn field_type(
    chunk: &ValidatedChunk,
    field: &StructuralFieldMetadata,
) -> Result<ExpectedType, SealedSemanticDagError> {
    if field.resource.is_some() {
        return Err(SealedSemanticDagError::UnsupportedValidatedType);
    }
    match field.route {
        StructuralFieldRoute::Structural(id) => {
            eligible_structural_type(chunk, id)?;
            Ok(ExpectedType::Structural(id))
        }
        StructuralFieldRoute::Copy => {
            let value_type = field
                .runtime_type
                .ok_or(SealedSemanticDagError::UnsupportedValidatedType)?;
            if !matches!(
                value_type.kind,
                StructuralKind::Unit
                    | StructuralKind::Bool
                    | StructuralKind::I64
                    | StructuralKind::F64
                    | StructuralKind::Static
            ) {
                return Err(SealedSemanticDagError::UnsupportedValidatedType);
            }
            Ok(ExpectedType::Inline(value_type))
        }
        StructuralFieldRoute::Unique
        | StructuralFieldRoute::Resource
        | StructuralFieldRoute::LegacyHeap => Err(SealedSemanticDagError::UnsupportedValidatedType),
    }
}

pub(super) fn runtime_type(
    chunk: &ValidatedChunk,
    expected: ExpectedType,
) -> Result<StructuralType, SealedSemanticDagError> {
    match expected {
        ExpectedType::Structural(id) => Ok(eligible_structural_type(chunk, id)?.runtime_type),
        ExpectedType::Inline(value_type) => Ok(value_type),
    }
}

pub(super) fn structural_type(
    chunk: &ValidatedChunk,
    id: StructuralTypeId,
) -> Result<&StructuralTypeMetadata, SealedSemanticDagError> {
    chunk
        .structural_types()
        .get_structural(id)
        .filter(|metadata| metadata.id == id)
        .ok_or(SealedSemanticDagError::ValidatedShapeMismatch)
}

fn eligible_structural_type(
    chunk: &ValidatedChunk,
    id: StructuralTypeId,
) -> Result<&StructuralTypeMetadata, SealedSemanticDagError> {
    let metadata = structural_type(chunk, id)?;
    if metadata.mode == StructuralTypeMode::Affine
        || metadata.runtime_type.kind == StructuralKind::ByteVector
    {
        return Err(SealedSemanticDagError::UnsupportedValidatedType);
    }
    Ok(metadata)
}

pub(super) fn dag_type(
    value_type: StructuralType,
) -> Result<SemanticDagType, SealedSemanticDagError> {
    let kind = match value_type.kind {
        StructuralKind::Unit => SemanticDagKind::Unit,
        StructuralKind::Bool => SemanticDagKind::Bool,
        StructuralKind::I64 => SemanticDagKind::I64,
        StructuralKind::F64 => SemanticDagKind::F64,
        StructuralKind::String => SemanticDagKind::String,
        StructuralKind::Path => SemanticDagKind::Path,
        StructuralKind::Product => SemanticDagKind::Product,
        StructuralKind::Enum => SemanticDagKind::Enum,
        StructuralKind::Static => SemanticDagKind::Static,
        StructuralKind::Bytes | StructuralKind::ByteVector => {
            return Err(SealedSemanticDagError::UnsupportedValidatedType)
        }
    };
    Ok(SemanticDagType::new(
        value_type.layout,
        value_type.semantic_type,
        kind,
    ))
}
