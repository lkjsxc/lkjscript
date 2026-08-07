use super::model::SegmentedListLocation;
use super::*;

pub(super) const SEGMENT_CAPACITY: usize = 1_024;

#[derive(Debug)]
pub(super) struct SegmentedListEntry<T> {
    pub(super) element: T,
    pub(super) tail: SegmentedListKey,
    pub(super) list_type: u64,
}

#[derive(Debug)]
pub(super) struct SegmentedListSegment<T> {
    pub(super) entries: Vec<SegmentedListEntry<T>>,
}

#[derive(Debug)]
pub struct SegmentedListArena<T> {
    pub(super) id: SegmentedListArenaId,
    pub(super) segments: Vec<SegmentedListSegment<T>>,
    pub(super) metrics: SegmentedListMetrics,
}

impl<T> SegmentedListArena<T> {
    pub fn new() -> Result<Self, SegmentedListError> {
        Ok(Self {
            id: SegmentedListArenaId::fresh()?,
            segments: Vec::new(),
            metrics: SegmentedListMetrics::default(),
        })
    }

    pub const fn id(&self) -> SegmentedListArenaId {
        self.id
    }

    pub const fn metrics(&self) -> SegmentedListMetrics {
        self.metrics
    }

    pub fn reserved_bytes_estimate(&self) -> Result<u64, SegmentedListError> {
        let segment_bytes = storage_bytes::<SegmentedListSegment<T>>(self.segments.capacity())?;
        self.segments
            .iter()
            .try_fold(segment_bytes, |total, segment| {
                total
                    .checked_add(storage_bytes::<SegmentedListEntry<T>>(
                        segment.entries.capacity(),
                    )?)
                    .ok_or(SegmentedListError::Limit(
                        SegmentedListLimit::Representation,
                    ))
            })
    }

    pub fn prepend_storage_increase(&self) -> Result<u64, SegmentedListError> {
        if self
            .segments
            .last()
            .is_none_or(|segment| segment.entries.len() == SEGMENT_CAPACITY)
        {
            storage_bytes::<SegmentedListSegment<T>>(1)?
                .checked_add(storage_bytes::<SegmentedListEntry<T>>(SEGMENT_CAPACITY)?)
                .ok_or(SegmentedListError::Limit(
                    SegmentedListLimit::Representation,
                ))
        } else {
            Ok(0)
        }
    }

    pub const fn empty(&self) -> SegmentedListKey {
        SegmentedListKey::empty(self.id)
    }

    pub fn key_from_word(&self, word: u64) -> Result<SegmentedListKey, SegmentedListError> {
        let key = SegmentedListKey::from_word(self.id, word)?;
        self.validate_key(key)?;
        Ok(key)
    }

    pub fn first(&mut self, key: SegmentedListKey) -> Result<&T, SegmentedListError> {
        let location = self.nonempty_location(key)?;
        self.metrics.first_reads = self.metrics.first_reads.saturating_add(1);
        Ok(&self.entry(location)?.element)
    }

    pub fn rest(&mut self, key: SegmentedListKey) -> Result<SegmentedListKey, SegmentedListError> {
        let location = self.nonempty_location(key)?;
        self.metrics.rest_reads = self.metrics.rest_reads.saturating_add(1);
        Ok(self.entry(location)?.tail)
    }

    pub fn view(
        &self,
        key: SegmentedListKey,
    ) -> Result<Option<(&T, SegmentedListKey)>, SegmentedListError> {
        self.validate_key(key)?;
        let Some(location) = key.location() else {
            return Ok(None);
        };
        let entry = self.entry(location)?;
        Ok(Some((&entry.element, entry.tail)))
    }

    pub fn validate_type(
        &self,
        key: SegmentedListKey,
        list_type: u64,
    ) -> Result<(), SegmentedListError> {
        self.validate_key(key)?;
        if let Some(location) = key.location() {
            if self.entry(location)?.list_type != list_type {
                return Err(SegmentedListError::WrongType);
            }
        }
        Ok(())
    }

    pub fn validate_key(&self, key: SegmentedListKey) -> Result<(), SegmentedListError> {
        if key.arena() != self.id {
            return Err(SegmentedListError::WrongArena);
        }
        if let Some(location) = key.location() {
            self.entry(location)?;
        }
        Ok(())
    }

    fn nonempty_location(
        &self,
        key: SegmentedListKey,
    ) -> Result<SegmentedListLocation, SegmentedListError> {
        self.validate_key(key)?;
        key.location().ok_or(SegmentedListError::EmptyList)
    }

    pub(super) fn entry(
        &self,
        location: SegmentedListLocation,
    ) -> Result<&SegmentedListEntry<T>, SegmentedListError> {
        self.segments
            .get(usize::from(location.segment))
            .and_then(|segment| segment.entries.get(usize::from(location.entry)))
            .ok_or(SegmentedListError::InvalidKey)
    }
}

fn storage_bytes<T>(count: usize) -> Result<u64, SegmentedListError> {
    u64::try_from(count)
        .ok()
        .zip(u64::try_from(std::mem::size_of::<T>()).ok())
        .and_then(|(count, item)| count.checked_mul(item))
        .ok_or(SegmentedListError::Limit(
            SegmentedListLimit::Representation,
        ))
}
