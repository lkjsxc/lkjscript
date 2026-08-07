use super::arena::{SegmentedListEntry, SegmentedListSegment, SEGMENT_CAPACITY};
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
        let needs_segment = self
            .segments
            .last()
            .is_none_or(|segment| segment.entries.len() == SEGMENT_CAPACITY);
        if needs_segment {
            self.add_segment()?;
        }
        let segment_index = u16::try_from(self.segments.len() - 1)
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::Representation))?;
        let segment = self
            .segments
            .last_mut()
            .ok_or(SegmentedListError::InvalidKey)?;
        let entry_index = u16::try_from(segment.entries.len())
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::Representation))?;
        segment.entries.push(SegmentedListEntry {
            element,
            tail,
            list_type,
        });
        self.metrics.live_entries =
            self.metrics
                .live_entries
                .checked_add(1)
                .ok_or(SegmentedListError::Limit(
                    SegmentedListLimit::Representation,
                ))?;
        self.metrics.prepends = self.metrics.prepends.saturating_add(1);
        Ok(SegmentedListKey::new(self.id, segment_index, entry_index))
    }

    fn add_segment(&mut self) -> Result<(), SegmentedListError> {
        // The packed key currently has sixteen segment bits. This is an encoded
        // representation boundary, not an execution-policy count.
        u16::try_from(self.segments.len())
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::Representation))?;
        self.segments
            .try_reserve(1)
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::HostAllocation))?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(SEGMENT_CAPACITY)
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::HostAllocation))?;
        self.segments.push(SegmentedListSegment { entries });
        self.metrics.live_segments =
            self.metrics
                .live_segments
                .checked_add(1)
                .ok_or(SegmentedListError::Limit(
                    SegmentedListLimit::Representation,
                ))?;
        self.metrics.segment_allocations = self.metrics.segment_allocations.saturating_add(1);
        Ok(())
    }
}
