use crate::*;

impl JitValueServices<'_> {
    pub(crate) fn execute_equality(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        let descriptor = site.descriptor();
        let _result_type = descriptor.result_type();
        let argument = |index: usize| {
            arguments
                .get(index)
                .copied()
                .ok_or(NativeServiceError::HostFailure)
        };
        let _as_i64 = |value: NativeValue| match value {
            NativeValue::I64(value) => Ok(value),
            _ => Err(NativeServiceError::HostFailure),
        };
        let _as_f64 = |value: NativeValue| match value {
            NativeValue::F64Bits(bits) => Ok(f64::from_bits(bits)),
            _ => Err(NativeServiceError::HostFailure),
        };
        let as_reference = |value: NativeValue| match value {
            NativeValue::Reference(reference) => Ok(reference),
            _ => Err(NativeServiceError::HostFailure),
        };
        match descriptor.operation() {
            HeapOperation::ListEqual => {
                let left = as_reference(argument(0)?)?;
                let right = as_reference(argument(1)?)?;
                if left.reference_type() != right.reference_type() {
                    return self.trap("list-equal layout mismatch");
                }
                let left = self.list_key(left)?;
                let right = self.list_key(right)?;
                self.segmented_list_equal(left, right)
                    .map(NativeValue::Bool)
            }
            _ => Err(NativeServiceError::HostFailure),
        }
    }

    fn segmented_list_equal(
        &mut self,
        mut left: lkjscript_core::SegmentedListKey,
        mut right: lkjscript_core::SegmentedListKey,
    ) -> Result<bool, NativeServiceError> {
        let mut steps = 0_usize;
        loop {
            let left_view = self
                .lists
                .view(left)
                .map_err(|_| NativeServiceError::Trap)?;
            let right_view = self
                .lists
                .view(right)
                .map_err(|_| NativeServiceError::Trap)?;
            let (Some((left_value, left_tail)), Some((right_value, right_tail))) =
                (left_view, right_view)
            else {
                return Ok(left_view.is_none() && right_view.is_none());
            };
            if steps >= MAX_LIST_EQUAL_STEPS {
                return self.trap("list-equal step limit exceeded");
            }
            if !scalar_value_equal(*left_value, *right_value).ok_or_else(|| {
                self.last_trap = Some("list-equal element type mismatch".into());
                NativeServiceError::Trap
            })? {
                return Ok(false);
            }
            left = left_tail;
            right = right_tail;
            steps = steps.saturating_add(1);
        }
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
        return Some(left.as_f64()? == right.as_f64()?);
    }
    if left.as_static_string().is_some() || right.as_static_string().is_some() {
        return Some(left.as_static_string()? == right.as_static_string()?);
    }
    None
}
