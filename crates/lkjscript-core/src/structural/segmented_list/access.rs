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
        max_steps: u32,
        mut equal: impl FnMut(&T, &T) -> bool,
    ) -> Result<bool, SegmentedListError> {
        self.validate_key(left)?;
        self.validate_key(right)?;
        let mut steps = 0_u32;
        loop {
            match (left.location(), right.location()) {
                (None, None) => return Ok(true),
                (None, Some(_)) | (Some(_), None) => return Ok(false),
                (Some(left_location), Some(right_location)) => {
                    if steps >= max_steps {
                        return Err(SegmentedListError::Limit(
                            SegmentedListLimit::TraversalSteps,
                        ));
                    }
                    let left_entry = self.entry(left_location)?;
                    let right_entry = self.entry(right_location)?;
                    if !equal(&left_entry.element, &right_entry.element) {
                        return Ok(false);
                    }
                    left = left_entry.tail;
                    right = right_entry.tail;
                    steps = steps.checked_add(1).ok_or(SegmentedListError::Limit(
                        SegmentedListLimit::TraversalSteps,
                    ))?;
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

    pub fn collect_cloned(
        &self,
        mut key: SegmentedListKey,
        max_steps: Option<u64>,
    ) -> Result<Vec<T>, SegmentedListError> {
        self.validate_key(key)?;
        let mut output = Vec::new();
        while let Some(location) = key.location() {
            if max_steps.is_some_and(|maximum| output.len() as u64 >= maximum) {
                return Err(SegmentedListError::Limit(
                    SegmentedListLimit::TraversalSteps,
                ));
            }
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
