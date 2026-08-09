use std::sync::Arc;

use super::model::{EntityAddress, SnapshotIndexes};
use super::program::SemanticProgram;
use super::{EntityId, WorkspaceError};

/// Source-independent type input for the implemented semantic-construction surface.
///
/// Nominal cases use stable workspace identities. Compiler-local product, enum,
/// layout, and source identities never cross this boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum SemanticTypeRef {
    Unit,
    Bool,
    I64,
    F64,
    Bytes,
    ByteVector,
    ByteSlice,
    ByteSliceMut,
    Product(EntityId),
    Enum(EntityId),
}

/// Machine-readable selected type facts. Unsupported imported/generic forms are
/// explicit rather than silently reduced to display text.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SemanticTypeView {
    Known(SemanticTypeRef),
    Unsupported {
        display: Arc<str>,
        nominal: Option<EntityId>,
    },
}

impl SemanticTypeView {
    pub fn known(&self) -> Option<SemanticTypeRef> {
        match self {
            Self::Known(ty) => Some(*ty),
            Self::Unsupported { .. } => None,
        }
    }
}

pub(super) fn view(
    _program: &SemanticProgram,
    indexes: &SnapshotIndexes,
    ty: &crate::Type,
) -> Result<SemanticTypeView, WorkspaceError> {
    let known = match ty {
        crate::Type::Unit => Some(SemanticTypeRef::Unit),
        crate::Type::Bool => Some(SemanticTypeRef::Bool),
        crate::Type::I64 => Some(SemanticTypeRef::I64),
        crate::Type::F64 => Some(SemanticTypeRef::F64),
        crate::Type::Bytes => Some(SemanticTypeRef::Bytes),
        crate::Type::ByteVector => Some(SemanticTypeRef::ByteVector),
        crate::Type::ByteSlice => Some(SemanticTypeRef::ByteSlice),
        crate::Type::ByteSliceMut => Some(SemanticTypeRef::ByteSliceMut),
        crate::Type::Product(name) => {
            let index = indexes
                .product_name_indices
                .get(name)
                .copied()
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product type")))?;
            let raw = u64::try_from(index)
                .map_err(|_| WorkspaceError::Host(Arc::from("product type index exceeds u64")))?;
            indexes
                .address_entities
                .get(&EntityAddress::Product(raw))
                .copied()
                .map(SemanticTypeRef::Product)
        }
        crate::Type::Enum { id, arguments, .. } => {
            let index = indexes
                .enum_identity_indices
                .get(id)
                .copied()
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum type")))?;
            let raw = u64::try_from(index)
                .map_err(|_| WorkspaceError::Host(Arc::from("enum type index exceeds u64")))?;
            let nominal = indexes
                .address_entities
                .get(&EntityAddress::Enum(raw))
                .copied();
            if arguments.is_empty() {
                nominal.map(SemanticTypeRef::Enum)
            } else {
                return Ok(SemanticTypeView::Unsupported {
                    display: Arc::from(ty.to_string()),
                    nominal,
                });
            }
        }
        _ => None,
    };
    Ok(match known {
        Some(ty) => SemanticTypeView::Known(ty),
        None => SemanticTypeView::Unsupported {
            display: Arc::from(ty.to_string()),
            nominal: None,
        },
    })
}
