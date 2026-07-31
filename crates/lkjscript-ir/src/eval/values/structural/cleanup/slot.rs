use crate::eval::{EvalValue, Flow};

pub(crate) fn restore_slot(
    values: &mut [Option<EvalValue>],
    id: crate::ValueId,
    value: EvalValue,
) -> Result<(), Flow> {
    let slot = values
        .get_mut(id.index().unwrap_or(usize::MAX))
        .ok_or_else(|| Flow::Trap("evaluator restore ValueId is out of range".into()))?;
    if slot.is_some() {
        return Err(Flow::Trap("evaluator restore slot is occupied".into()));
    }
    *slot = Some(value);
    Ok(())
}
