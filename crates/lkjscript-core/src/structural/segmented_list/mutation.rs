use super::arena::{SegmentedListEntry, SegmentedListSegment};
use super::*;

impl<T> SegmentedListArena<T> {
    pub fn prepend(
        &mut self,
        element: T,
        tail: SegmentedListKey,
    ) -> Result<SegmentedListKey, SegmentedListError> {
        self.prepend_typed(element, tail, 0)
    }

    pub fn prepend_typed(
        &mut self,
        element: T,
        tail: SegmentedListKey,
        list_type: u64,
    ) -> Result<SegmentedListKey, SegmentedListError> {
        self.validate_type(tail, list_type)?;
        if self.metrics.live_entries >= self.limits.max_entries.get() {
            return Err(SegmentedListError::Limit(SegmentedListLimit::Entries));
        }
        let capacity = usize::from(self.limits.segment_capacity.get());
        let needs_segment = self
            .segments
            .last()
            .is_none_or(|segment| segment.entries.len() == capacity);
        if needs_segment {
            self.add_segment(capacity)?;
        }
        let segment_index = u32::try_from(self.segments.len().saturating_sub(1))
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::Segments))?;
        let segment = self
            .segments
            .last_mut()
            .ok_or(SegmentedListError::InvalidKey)?;
        let entry_index = u16::try_from(segment.entries.len())
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::Entries))?;
        segment.entries.push(SegmentedListEntry {
            element,
            tail,
            list_type,
        });
        self.metrics.live_entries = self
            .metrics
            .live_entries
            .checked_add(1)
            .ok_or(SegmentedListError::Limit(SegmentedListLimit::Entries))?;
        self.metrics.prepends = self.metrics.prepends.saturating_add(1);
        Ok(SegmentedListKey::new(self.id, segment_index, entry_index))
    }

    fn add_segment(&mut self, capacity: usize) -> Result<(), SegmentedListError> {
        if self.metrics.live_segments >= self.limits.max_segments.get() {
            return Err(SegmentedListError::Limit(SegmentedListLimit::Segments));
        }
        self.segments
            .try_reserve(1)
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::HostAllocation))?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(capacity)
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::HostAllocation))?;
        self.segments.push(SegmentedListSegment { entries });
        self.metrics.live_segments = self
            .metrics
            .live_segments
            .checked_add(1)
            .ok_or(SegmentedListError::Limit(SegmentedListLimit::Segments))?;
        self.metrics.segment_allocations = self.metrics.segment_allocations.saturating_add(1);
        Ok(())
    }
}
