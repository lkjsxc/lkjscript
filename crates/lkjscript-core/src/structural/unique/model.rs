use std::collections::HashMap;
use std::num::{NonZeroU64, TryFromIntError};
use std::sync::atomic::{AtomicU64, Ordering};

use super::object::Slot;
use super::{InvalidUniqueKeyWord, UniqueStoreError, UniqueStoreLeak};

static NEXT_UNIQUE_TOKEN: AtomicU64 = AtomicU64::new(1);

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

/// One opaque, runtime-local, nonzero ABI token.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct UniqueKeyWord(NonZeroU64);

impl UniqueKeyWord {
    pub const fn new(word: u64) -> Result<Self, InvalidUniqueKeyWord> {
        match NonZeroU64::new(word) {
            Some(word) => Ok(Self(word)),
            None => Err(InvalidUniqueKeyWord::ZeroToken),
        }
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }

    pub(super) const fn from_nonzero(word: NonZeroU64) -> Self {
        Self(word)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct RawUniqueKey {
    pub(super) store: UniqueStoreId,
    pub(super) index: u64,
    pub(super) generation: NonZeroU64,
    pub(super) word: UniqueKeyWord,
}

macro_rules! typed_key {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub struct $name(RawUniqueKey);

        impl $name {
            pub(super) const fn from_raw(raw: RawUniqueKey) -> Self {
                Self(raw)
            }

            pub const fn opaque_word(self) -> UniqueKeyWord {
                self.0.word
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
    pub live_objects: u64,
    pub peak_live_objects: u64,
    pub live_bytes: u64,
    pub peak_live_bytes: u64,
    pub reused_slots: u64,
    pub retired_slots: u64,
    pub stale_failures: u64,
    pub wrong_layout_failures: u64,
    pub allocated_bytes: u64,
}

#[derive(Debug)]
pub struct UniqueStore {
    pub(super) id: UniqueStoreId,
    pub(super) slots: Vec<Slot>,
    pub(super) free_head: Option<u64>,
    pub(super) tokens: HashMap<UniqueKeyWord, RawUniqueKey>,
    pub(super) stats: UniqueStoreStats,
}

impl UniqueStore {
    pub fn new(id: UniqueStoreId) -> Self {
        Self {
            id,
            slots: Vec::new(),
            free_head: None,
            tokens: HashMap::new(),
            stats: UniqueStoreStats::default(),
        }
    }

    pub const fn id(&self) -> UniqueStoreId {
        self.id
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

    pub(super) fn next_token(&mut self) -> Result<UniqueKeyWord, UniqueStoreError> {
        self.tokens
            .try_reserve(1)
            .map_err(|_| UniqueStoreError::StorageCapacity)?;
        let raw = NEXT_UNIQUE_TOKEN
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                next.checked_add(1)
            })
            .map_err(|_| UniqueStoreError::RepresentationExhausted)?;
        let token = NonZeroU64::new(raw).ok_or(UniqueStoreError::RepresentationExhausted)?;
        Ok(UniqueKeyWord::from_nonzero(token))
    }

    pub(super) fn bind(&mut self, word: UniqueKeyWord) -> Result<RawUniqueKey, UniqueStoreError> {
        self.tokens.get(&word).copied().ok_or_else(|| {
            self.stats.stale_failures = self.stats.stale_failures.saturating_add(1);
            UniqueStoreError::StaleKey
        })
    }
}

impl From<TryFromIntError> for UniqueStoreError {
    fn from(_: TryFromIntError) -> Self {
        Self::ArithmeticOverflow
    }
}
