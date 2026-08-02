use crate::*;

impl JitIslandServices {
    pub(super) fn compare_lists(
        &mut self,
        left: lkjscript_core::SegmentedListKey,
        right: lkjscript_core::SegmentedListKey,
    ) -> Result<bool, NativeServiceError> {
        let mut pending = Vec::new();
        pending
            .try_reserve(1)
            .map_err(|_| NativeServiceError::ResourceLimitExceeded)?;
        pending.push((left, right));
        let mut steps = 0_usize;
        while let Some((mut left, mut right)) = pending.pop() {
            loop {
                let left_view = self
                    .lists
                    .view(left)
                    .map_err(Self::map_list_error)?
                    .map(|(value, tail)| (*value, tail));
                let right_view = self
                    .lists
                    .view(right)
                    .map_err(Self::map_list_error)?
                    .map(|(value, tail)| (*value, tail));
                let (Some((left_value, left_tail)), Some((right_value, right_tail))) =
                    (left_view, right_view)
                else {
                    if left_view.is_some() != right_view.is_some() {
                        return Ok(false);
                    }
                    break;
                };
                if steps >= MAX_LIST_EQUAL_STEPS {
                    self.structural
                        .record_trap("nested structural list equality step limit exceeded");
                    return Err(NativeServiceError::Trap);
                }
                steps = steps.saturating_add(1);
                if left_value.as_structural_root().is_some()
                    || right_value.as_structural_root().is_some()
                {
                    if !self.structural.list_owners_equal(left_value, right_value)? {
                        return Ok(false);
                    }
                } else {
                    let nested = (
                        self.nested_list_key(left_value),
                        self.nested_list_key(right_value),
                    );
                    match nested {
                        (Some(Ok(left)), Some(Ok(right))) => {
                            pending
                                .try_reserve(1)
                                .map_err(|_| NativeServiceError::ResourceLimitExceeded)?;
                            pending.push((left, right));
                        }
                        (None, None)
                            if scalar_value_equal(left_value, right_value) == Some(true) => {}
                        _ => return Ok(false),
                    }
                }
                left = left_tail;
                right = right_tail;
            }
        }
        Ok(true)
    }

    fn nested_list_key(
        &self,
        value: Value,
    ) -> Option<Result<lkjscript_core::SegmentedListKey, NativeServiceError>> {
        let word = if value.is_empty_list() {
            0
        } else {
            value.as_segmented_list()?
        };
        Some(self.lists.key_from_word(word).map_err(Self::map_list_error))
    }
}

fn scalar_value_equal(left: Value, right: Value) -> Option<bool> {
    if left.is_unit() || right.is_unit() {
        return (left.is_unit() && right.is_unit()).then_some(true);
    }
    if left.as_bool().is_some() || right.as_bool().is_some() {
        return Some(left.as_bool()? == right.as_bool()?);
    }
    if left.as_i64().is_some() || right.as_i64().is_some() {
        return Some(left.as_i64()? == right.as_i64()?);
    }
    if left.as_f64().is_some() || right.as_f64().is_some() {
        return Some(left.as_f64_bits()? == right.as_f64_bits()?);
    }
    None
}
