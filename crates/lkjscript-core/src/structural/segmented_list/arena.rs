use super::model::SegmentedListLocation;
use super::*;

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
    pub(super) limits: SegmentedListArenaLimits,
    pub(super) segments: Vec<SegmentedListSegment<T>>,
    pub(super) metrics: SegmentedListMetrics,
}

impl<T> SegmentedListArena<T> {
    pub fn new(limits: SegmentedListArenaLimits) -> Result<Self, SegmentedListError> {
        if limits.max_segments.get() > u32::from(u16::MAX) + 1 {
            return Err(SegmentedListError::InvalidLimits);
        }
        Ok(Self {
            id: SegmentedListArenaId::fresh()?,
            limits,
            segments: Vec::new(),
            metrics: SegmentedListMetrics::default(),
        })
    }

    pub const fn id(&self) -> SegmentedListArenaId {
        self.id
    }

    pub const fn limits(&self) -> SegmentedListArenaLimits {
        self.limits
    }

    pub const fn metrics(&self) -> SegmentedListMetrics {
        self.metrics
    }

    pub fn reserved_bytes_estimate(&self) -> u64 {
        u64::from(self.metrics.live_segments).saturating_mul(self.segment_storage_bytes())
    }

    pub fn prepend_storage_increase(&self) -> u64 {
        let capacity = usize::from(self.limits.segment_capacity.get());
        if self
            .segments
            .last()
            .is_none_or(|segment| segment.entries.len() == capacity)
        {
            self.segment_storage_bytes()
        } else {
            0
        }
    }

    fn segment_storage_bytes(&self) -> u64 {
        let segment_bytes = std::mem::size_of::<SegmentedListSegment<T>>() as u64;
        let entry_bytes = std::mem::size_of::<SegmentedListEntry<T>>() as u64;
        let capacity = u64::from(self.limits.segment_capacity.get());
        segment_bytes.saturating_add(entry_bytes.saturating_mul(capacity))
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
            .get(location.segment as usize)
            .and_then(|segment| segment.entries.get(usize::from(location.entry)))
            .ok_or(SegmentedListError::InvalidKey)
    }
}
