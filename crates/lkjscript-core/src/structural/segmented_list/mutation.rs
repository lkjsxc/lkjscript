use super::arena::{SegmentedListEntry, SegmentedListSegment, SEGMENT_CAPACITY};
use super::model::{next_list_token, SegmentedListLocation};
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
        let segment_index = if needs_segment {
            u64::try_from(self.segments.len())
        } else {
            u64::try_from(self.segments.len() - 1)
        }
        .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::Representation))?;
        let entry_index = if needs_segment {
            0
        } else {
            u64::try_from(
                self.segments
                    .last()
                    .ok_or(SegmentedListError::InvalidKey)?
                    .entries
                    .len(),
            )
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::Representation))?
        };
        let next_live_entries =
            self.metrics
                .live_entries
                .checked_add(1)
                .ok_or(SegmentedListError::Limit(
                    SegmentedListLimit::Representation,
                ))?;
        self.locations
            .try_reserve(1)
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::HostAllocation))?;
        let token = next_list_token()?;
        if needs_segment {
            self.add_segment()?;
        } else {
            self.segments
                .last_mut()
                .ok_or(SegmentedListError::InvalidKey)?
                .entries
                .try_reserve(1)
                .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::HostAllocation))?;
        }
        let segment = self
            .segments
            .last_mut()
            .ok_or(SegmentedListError::InvalidKey)?;
        segment.entries.push(SegmentedListEntry {
            element,
            tail,
            list_type,
        });
        let location = SegmentedListLocation {
            segment: segment_index,
            entry: entry_index,
        };
        if self.locations.insert(token.get(), location).is_some() {
            return Err(SegmentedListError::InvalidKey);
        }
        self.metrics.live_entries = next_live_entries;
        self.metrics.prepends = self.metrics.prepends.saturating_add(1);
        Ok(SegmentedListKey::new(self.id, token))
    }

    fn add_segment(&mut self) -> Result<(), SegmentedListError> {
        let next_live_segments =
            self.metrics
                .live_segments
                .checked_add(1)
                .ok_or(SegmentedListError::Limit(
                    SegmentedListLimit::Representation,
                ))?;
        self.segments
            .try_reserve(1)
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::HostAllocation))?;
        let mut entries = Vec::new();
        entries
            .try_reserve(1)
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::HostAllocation))?;
        self.segments.push(SegmentedListSegment { entries });
        self.metrics.live_segments = next_live_segments;
        self.metrics.segment_allocations = self.metrics.segment_allocations.saturating_add(1);
        Ok(())
    }
}
