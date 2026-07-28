use std::num::{NonZeroU32, NonZeroU64};

use super::object::Slot;
use super::{InvalidUniqueKeyWord, UniqueStoreError, UniqueStoreLeak, UniqueStoreLimits};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UniqueStoreId(NonZeroU64);

impl UniqueStoreId {
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn get(self) -> NonZeroU64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UniqueLayout {
    ByteVector,
    Bytes,
    Path,
}

/// A runtime-local key projection containing only slot index and generation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct UniqueKeyWord(u64);

impl UniqueKeyWord {
    const INDEX_BITS: u32 = u32::BITS;

    pub const fn new(word: u64) -> Result<Self, InvalidUniqueKeyWord> {
        let generation = (word >> Self::INDEX_BITS) as u32;
        if generation == 0 {
            return Err(InvalidUniqueKeyWord::ZeroGeneration);
        }
        Ok(Self(word))
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub(super) const fn from_raw(raw: RawUniqueKey) -> Self {
        let generation = (raw.generation.get() as u64) << Self::INDEX_BITS;
        Self(generation | raw.index as u64)
    }

    pub(super) fn bind(self, store: UniqueStoreId) -> Result<RawUniqueKey, UniqueStoreError> {
        let generation = (self.0 >> Self::INDEX_BITS) as u32;
        let generation = NonZeroU32::new(generation).ok_or(UniqueStoreError::ArithmeticOverflow)?;
        Ok(RawUniqueKey {
            store,
            index: self.0 as u32,
            generation,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct RawUniqueKey {
    pub(super) store: UniqueStoreId,
    pub(super) index: u32,
    pub(super) generation: NonZeroU32,
}

macro_rules! typed_key {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub struct $name(RawUniqueKey);

        impl $name {
            pub(super) const fn from_raw(raw: RawUniqueKey) -> Self {
                Self(raw)
            }

            pub const fn packed_word(self) -> UniqueKeyWord {
                UniqueKeyWord::from_raw(self.0)
            }

            pub(super) const fn raw(self) -> RawUniqueKey {
                self.0
            }
        }
    };
}

typed_key!(ByteVectorKey);
typed_key!(BytesKey);
typed_key!(PathKey);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticBytes(&'static [u8]);

impl StaticBytes {
    pub const fn new(bytes: &'static [u8]) -> Self {
        Self(bytes)
    }

    pub const fn as_slice(self) -> &'static [u8] {
        self.0
    }

    pub const fn len(self) -> usize {
        self.0.len()
    }

    pub const fn is_empty(self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UniqueStoreStats {
    pub allocations: u64,
    pub frees: u64,
    pub transfers: u64,
    pub live_objects: u32,
    pub peak_live_objects: u32,
    pub live_bytes: u64,
    pub peak_live_bytes: u64,
    pub reused_slots: u64,
    pub retired_slots: u32,
    pub stale_failures: u64,
    pub wrong_layout_failures: u64,
    pub allocated_bytes: u64,
}

#[derive(Debug)]
pub struct UniqueStore {
    pub(super) id: UniqueStoreId,
    pub(super) limits: UniqueStoreLimits,
    pub(super) slots: Vec<Slot>,
    pub(super) free_head: Option<u32>,
    pub(super) stats: UniqueStoreStats,
}

impl UniqueStore {
    pub const fn new(id: UniqueStoreId, limits: UniqueStoreLimits) -> Self {
        Self {
            id,
            limits,
            slots: Vec::new(),
            free_head: None,
            stats: UniqueStoreStats {
                allocations: 0,
                frees: 0,
                transfers: 0,
                live_objects: 0,
                peak_live_objects: 0,
                live_bytes: 0,
                peak_live_bytes: 0,
                reused_slots: 0,
                retired_slots: 0,
                stale_failures: 0,
                wrong_layout_failures: 0,
                allocated_bytes: 0,
            },
        }
    }

    pub const fn id(&self) -> UniqueStoreId {
        self.id
    }

    pub const fn limits(&self) -> UniqueStoreLimits {
        self.limits
    }

    pub const fn stats(&self) -> UniqueStoreStats {
        self.stats
    }

    pub const fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub const fn assert_no_leaks(&self) -> Result<(), UniqueStoreLeak> {
        if self.stats.live_objects == 0 && self.stats.live_bytes == 0 {
            Ok(())
        } else {
            Err(UniqueStoreLeak {
                live_objects: self.stats.live_objects,
                live_bytes: self.stats.live_bytes,
            })
        }
    }
}
