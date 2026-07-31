use super::{Value, ValueKind};

impl Value {
    pub const fn from_segmented_list(word: u64) -> Self {
        Self::new(ValueKind::SegmentedList, word)
    }

    pub const fn as_segmented_list(self) -> Option<u64> {
        match self.kind {
            ValueKind::SegmentedList => Some(self.payload),
            _ => None,
        }
    }

    pub(crate) const fn from_owned_list(index: u32) -> Self {
        Self::new(ValueKind::OwnedList, index as u64)
    }

    pub(crate) const fn as_owned_list(self) -> Option<u32> {
        match self.kind {
            ValueKind::OwnedList => Some(self.payload as u32),
            _ => None,
        }
    }
}
