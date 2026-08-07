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

    pub(crate) const fn from_owned_list(index: u64) -> Self {
        Self::new(ValueKind::OwnedList, index)
    }

    pub(crate) const fn as_owned_list(self) -> Option<u64> {
        match self.kind {
            ValueKind::OwnedList => Some(self.payload),
            _ => None,
        }
    }
}
