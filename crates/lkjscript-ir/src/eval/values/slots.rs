use super::super::*;

pub(crate) fn value(
    values: &[Option<EvalValue>],
    id: ValueId,
) -> std::result::Result<&EvalValue, Flow> {
    values
        .get(id.index().unwrap_or(usize::MAX))
        .and_then(Option::as_ref)
        .ok_or_else(|| Flow::Trap(format!("evaluator missing SSA value {}", id.raw())))
}

pub(crate) fn take_value(
    values: &mut [Option<EvalValue>],
    id: ValueId,
) -> std::result::Result<EvalValue, Flow> {
    values
        .get_mut(id.index().unwrap_or(usize::MAX))
        .and_then(Option::take)
        .ok_or_else(|| Flow::Trap(format!("evaluator moved or missing SSA value {}", id.raw())))
}

pub(crate) fn set_value(
    values: &mut [Option<EvalValue>],
    id: ValueId,
    value: EvalValue,
) -> std::result::Result<(), Flow> {
    let Some(slot) = values.get_mut(id.index().unwrap_or(usize::MAX)) else {
        return Err(Flow::Trap("evaluator ValueId is out of range".into()));
    };
    *slot = Some(value);
    Ok(())
}

pub(crate) fn values_for(
    values: &[Option<EvalValue>],
    ids: &[ValueId],
) -> std::result::Result<Vec<EvalValue>, Flow> {
    ids.iter().map(|id| value(values, *id).cloned()).collect()
}

pub(crate) fn values_for_call(
    values: &mut [Option<EvalValue>],
    ids: &[ValueId],
) -> std::result::Result<Vec<EvalValue>, Flow> {
    ids.iter()
        .map(|id| match value(values, *id)? {
            EvalValue::ByteVector(_) => take_value(values, *id),
            other => Ok(other.clone()),
        })
        .collect()
}

pub(crate) fn values_for_edge(
    values: &mut [Option<EvalValue>],
    ids: &[ValueId],
) -> std::result::Result<Vec<EvalValue>, Flow> {
    values_for_call(values, ids)
}

pub(crate) fn assign_parameters(
    values: &mut [Option<EvalValue>],
    parameters: &[crate::BlockParameter],
    arguments: Vec<EvalValue>,
) -> std::result::Result<(), Flow> {
    if parameters.len() != arguments.len() {
        return Err(Flow::Trap("evaluator block argument arity mismatch".into()));
    }
    for (parameter, argument) in parameters.iter().zip(arguments) {
        set_value(values, parameter.id, argument)?;
    }
    Ok(())
}
