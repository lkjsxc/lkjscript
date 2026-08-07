use super::*;

impl<T> SegmentedListArena<T> {
    pub fn is_empty(&self, key: SegmentedListKey) -> Result<bool, SegmentedListError> {
        self.validate_key(key)?;
        Ok(key.is_empty())
    }

    pub fn equal_by(
        &mut self,
        mut left: SegmentedListKey,
        mut right: SegmentedListKey,
        mut equal: impl FnMut(&T, &T) -> bool,
    ) -> Result<bool, SegmentedListError> {
        self.validate_key(left)?;
        self.validate_key(right)?;
        loop {
            match (left.location(), right.location()) {
                (None, None) => return Ok(true),
                (None, Some(_)) | (Some(_), None) => return Ok(false),
                (Some(left_location), Some(right_location)) => {
                    let left_entry = self.entry(left_location)?;
                    let right_entry = self.entry(right_location)?;
                    if !equal(&left_entry.element, &right_entry.element) {
                        return Ok(false);
                    }
                    left = left_entry.tail;
                    right = right_entry.tail;
                }
            }
        }
    }
}

impl<T: Clone> SegmentedListArena<T> {
    pub fn append_cloned_elements(&self, output: &mut Vec<T>) -> Result<(), SegmentedListError> {
        output
            .try_reserve(self.metrics().live_entries as usize)
            .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::HostAllocation))?;
        for segment in &self.segments {
            output.extend(segment.entries.iter().map(|entry| entry.element.clone()));
        }
        Ok(())
    }

    pub fn first_cloned(&mut self, key: SegmentedListKey) -> Result<T, SegmentedListError> {
        self.first(key).cloned()
    }

    pub fn collect_cloned(&self, mut key: SegmentedListKey) -> Result<Vec<T>, SegmentedListError> {
        self.validate_key(key)?;
        let mut output = Vec::new();
        while let Some(location) = key.location() {
            output
                .try_reserve(1)
                .map_err(|_| SegmentedListError::Limit(SegmentedListLimit::HostAllocation))?;
            let entry = self.entry(location)?;
            output.push(entry.element.clone());
            key = entry.tail;
        }
        Ok(output)
    }
}
