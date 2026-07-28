use std::num::NonZeroU32;

use super::{UniqueLayout, UniqueStoreError};

#[derive(Debug)]
pub(super) enum Payload {
    ByteVector(Vec<u8>),
    Bytes(Vec<u8>),
    Path(Box<[u8]>),
}

impl Payload {
    pub(super) const fn layout(&self) -> UniqueLayout {
        match self {
            Self::ByteVector(_) => UniqueLayout::ByteVector,
            Self::Bytes(_) => UniqueLayout::Bytes,
            Self::Path(_) => UniqueLayout::Path,
        }
    }

    pub(super) fn retained_bytes(&self) -> Result<u64, UniqueStoreError> {
        let retained = match self {
            Self::ByteVector(bytes) | Self::Bytes(bytes) => bytes.capacity(),
            Self::Path(bytes) => bytes.len(),
        };
        u64::try_from(retained).map_err(|_| UniqueStoreError::ArithmeticOverflow)
    }

    pub(super) fn freeze(&mut self) {
        if let Self::ByteVector(bytes) = self {
            *self = Self::Bytes(std::mem::take(bytes));
        }
    }

    pub(super) fn thaw(&mut self) {
        if let Self::Bytes(bytes) = self {
            *self = Self::ByteVector(std::mem::take(bytes));
        }
    }
}

#[derive(Debug)]
pub(super) enum SlotState {
    Occupied(Payload),
    Vacant { next: Option<u32> },
    Retired,
}

#[derive(Debug)]
pub(super) struct Slot {
    pub(super) generation: NonZeroU32,
    pub(super) state: SlotState,
}

impl Slot {
    pub(super) const fn occupied(generation: NonZeroU32, payload: Payload) -> Self {
        Self {
            generation,
            state: SlotState::Occupied(payload),
        }
    }
}
