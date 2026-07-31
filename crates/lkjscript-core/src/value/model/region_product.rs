use super::{Value, ValueKind};

impl Value {
    pub const fn from_region_product(key: crate::RegionProductKey) -> Self {
        Self::new(ValueKind::RegionProduct, key.to_word())
    }

    pub const fn as_region_product_word(self) -> Option<u64> {
        match self.kind {
            ValueKind::RegionProduct => Some(self.payload),
            _ => None,
        }
    }
}
