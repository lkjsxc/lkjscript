use crate::structural::{StructuralDestinationKey, StructuralValueKey, StructuralViewKey};

use super::{Value, ValueKind};

impl Value {
    #[doc(hidden)]
    pub const fn from_aggregate_adapter(word: u64) -> Self {
        Self::new(ValueKind::AggregateAdapter, word)
    }

    #[doc(hidden)]
    pub const fn as_aggregate_adapter(self) -> Option<u64> {
        match self.kind {
            ValueKind::AggregateAdapter => Some(self.payload),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub const fn from_structural_root(key: StructuralValueKey) -> Self {
        Self::new(ValueKind::StructuralRoot, key.get())
    }

    #[doc(hidden)]
    pub const fn as_structural_root(self) -> Option<StructuralValueKey> {
        match self.kind {
            ValueKind::StructuralRoot => StructuralValueKey::from_word(self.payload),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub const fn from_structural_view(key: StructuralViewKey) -> Self {
        Self::new(ValueKind::StructuralView, key.get())
    }

    #[doc(hidden)]
    pub const fn as_structural_view(self) -> Option<u64> {
        match self.kind {
            ValueKind::StructuralView => Some(self.payload),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub const fn from_structural_destination(key: StructuralDestinationKey) -> Self {
        Self::new(ValueKind::StructuralDestination, key.get())
    }

    #[doc(hidden)]
    pub const fn as_structural_destination(self) -> Option<u64> {
        match self.kind {
            ValueKind::StructuralDestination => Some(self.payload),
            _ => None,
        }
    }
}
