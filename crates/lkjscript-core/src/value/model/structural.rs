use crate::structural::StructuralValueKey;

use super::{Value, ValueKind};

impl Value {
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
}
