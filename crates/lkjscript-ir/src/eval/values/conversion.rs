use super::super::*;

pub(crate) fn as_bool(value: &EvalValue) -> std::result::Result<bool, Flow> {
    match value {
        EvalValue::Bool(value) => Ok(*value),
        _ => Err(Flow::Trap("expected Bool".into())),
    }
}

pub(crate) fn as_i64(value: &EvalValue) -> std::result::Result<i64, Flow> {
    match value {
        EvalValue::I64(value) => Ok(*value),
        _ => Err(Flow::Trap("expected I64".into())),
    }
}

pub(crate) fn as_f64_exact(value: &EvalValue) -> std::result::Result<f64, Flow> {
    match value {
        EvalValue::F64(value) => Ok(*value),
        _ => Err(Flow::Trap("expected F64".into())),
    }
}

pub(crate) fn as_numeric_f64(value: &EvalValue) -> std::result::Result<f64, Flow> {
    match value {
        EvalValue::I64(value) => Ok(*value as f64),
        EvalValue::F64(value) => Ok(*value),
        _ => Err(Flow::Trap("expected numeric value".into())),
    }
}

#[cfg(test)]
pub(crate) fn list_values_equal(
    left: &[EvalValue],
    right: &[EvalValue],
) -> std::result::Result<bool, Flow> {
    let mut index = 0_usize;
    loop {
        let left_head = left.get(index);
        let right_head = right.get(index);
        let (left_head, right_head) = match (left_head, right_head) {
            (None, None) => return Ok(true),
            (None, Some(_)) | (Some(_), None) => return Ok(false),
            (Some(left_head), Some(right_head)) => (left_head, right_head),
        };
        index = index
            .checked_add(1)
            .ok_or_else(|| Flow::Resource("heap bytes".into()))?;
        if !value_equal(left_head, right_head)? {
            return Ok(false);
        }
    }
}

pub(crate) fn index_value(value: &EvalValue, operation: &str) -> std::result::Result<usize, Flow> {
    usize::try_from(as_i64(value)?)
        .map_err(|_| Flow::Trap(format!("{operation} index out of range")))
}
