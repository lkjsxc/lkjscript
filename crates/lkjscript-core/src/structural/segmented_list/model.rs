use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ARENA_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_LIST_TOKEN: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SegmentedListArenaId(NonZeroU64);

impl SegmentedListArenaId {
    pub(super) fn fresh() -> Result<Self, SegmentedListError> {
        let id = NEXT_ARENA_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| SegmentedListError::IdentityExhausted)?;
        NonZeroU64::new(id)
            .map(Self)
            .ok_or(SegmentedListError::IdentityExhausted)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SegmentedListKey {
    arena: SegmentedListArenaId,
    token: Option<NonZeroU64>,
}

impl SegmentedListKey {
    pub const fn is_empty(self) -> bool {
        self.token.is_none()
    }

    pub const fn arena(self) -> SegmentedListArenaId {
        self.arena
    }

    pub const fn to_word(self) -> u64 {
        match self.token {
            Some(token) => token.get(),
            None => 0,
        }
    }

    pub(super) const fn empty(arena: SegmentedListArenaId) -> Self {
        Self { arena, token: None }
    }

    pub(super) const fn new(arena: SegmentedListArenaId, token: NonZeroU64) -> Self {
        Self {
            arena,
            token: Some(token),
        }
    }

    pub(super) const fn token(self) -> Option<NonZeroU64> {
        self.token
    }

    pub(super) fn from_word(
        arena: SegmentedListArenaId,
        word: u64,
    ) -> Result<Self, SegmentedListError> {
        if word == 0 {
            return Ok(Self::empty(arena));
        }
        NonZeroU64::new(word)
            .map(|token| Self::new(arena, token))
            .ok_or(SegmentedListError::InvalidKey)
    }
}

pub(super) fn next_list_token() -> Result<NonZeroU64, SegmentedListError> {
    let token = NEXT_LIST_TOKEN
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::Representation))?;
    NonZeroU64::new(token).ok_or(SegmentedListError::Limit(
        SegmentedListLimit::Representation,
    ))
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct SegmentedListLocation {
    pub segment: u64,
    pub entry: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SegmentedListMetrics {
    pub live_segments: u64,
    pub live_entries: u64,
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
