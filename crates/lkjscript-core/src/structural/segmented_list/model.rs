use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_ARENA_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SegmentedListArenaId(NonZeroU32);

impl SegmentedListArenaId {
    pub(super) fn fresh() -> Result<Self, SegmentedListError> {
        let id = NEXT_ARENA_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1).filter(|next| *next != 0)
            })
            .map_err(|_| SegmentedListError::IdentityExhausted)?;
        NonZeroU32::new(id)
            .map(Self)
            .ok_or(SegmentedListError::IdentityExhausted)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SegmentedListKey {
    arena: SegmentedListArenaId,
    location: Option<SegmentedListLocation>,
}

impl SegmentedListKey {
    pub const fn is_empty(self) -> bool {
        self.location.is_none()
    }

    pub const fn arena(self) -> SegmentedListArenaId {
        self.arena
    }

    pub const fn to_word(self) -> u64 {
        match self.location {
            None => 0,
            Some(location) => {
                ((self.arena.0.get() as u64) << 32)
                    | ((location.segment as u64) << 16)
                    | (location.entry as u64 + 1)
            }
        }
    }

    pub(super) const fn empty(arena: SegmentedListArenaId) -> Self {
        Self {
            arena,
            location: None,
        }
    }

    pub(super) const fn new(arena: SegmentedListArenaId, segment: u16, entry: u16) -> Self {
        Self {
            arena,
            location: Some(SegmentedListLocation { segment, entry }),
        }
    }

    pub(super) const fn location(self) -> Option<SegmentedListLocation> {
        self.location
    }

    pub(super) fn from_word(
        arena: SegmentedListArenaId,
        word: u64,
    ) -> Result<Self, SegmentedListError> {
        if word == 0 {
            return Ok(Self::empty(arena));
        }
        let encoded_arena =
            u32::try_from(word >> 32).map_err(|_| SegmentedListError::InvalidKey)?;
        if encoded_arena != arena.0.get() {
            return Err(SegmentedListError::WrongArena);
        }
        let segment = u16::try_from((word >> 16) & u64::from(u16::MAX))
            .map_err(|_| SegmentedListError::InvalidKey)?;
        let encoded_entry = u16::try_from(word & u64::from(u16::MAX))
            .map_err(|_| SegmentedListError::InvalidKey)?;
        let entry = encoded_entry
            .checked_sub(1)
            .ok_or(SegmentedListError::InvalidKey)?;
        Ok(Self::new(arena, segment, entry))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct SegmentedListLocation {
    pub segment: u16,
    pub entry: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SegmentedListMetrics {
    pub live_segments: u32,
    pub live_entries: u32,
    pub segment_allocations: u64,
    pub prepends: u64,
    pub first_reads: u64,
    pub rest_reads: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SegmentedListLimit {
    Representation,
    HostAllocation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SegmentedListError {
    WrongArena,
    WrongType,
    InvalidKey,
    EmptyList,
    IdentityExhausted,
    Limit(SegmentedListLimit),
}
