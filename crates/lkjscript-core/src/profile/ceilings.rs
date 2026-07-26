use crate::budget::{ResourceCategory, RESOURCE_CATEGORY_COUNT};

use super::ceiling_sets::MAXIMA;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceCeilings {
    pub(crate) values: [u64; RESOURCE_CATEGORY_COUNT],
}

impl ResourceCeilings {
    pub const fn limit(self, category: ResourceCategory) -> u64 {
        self.values[category.index()]
    }

    pub const fn implementation_maxima() -> Self {
        Self { values: MAXIMA }
    }

    pub(crate) fn digest(self) -> [u8; 32] {
        let mut encoded = [0_u8; RESOURCE_CATEGORY_COUNT * 8];
        for (slot, limit) in encoded.chunks_exact_mut(8).zip(self.values) {
            slot.copy_from_slice(&limit.to_be_bytes());
        }
        crate::sha256(&encoded)
    }
}
