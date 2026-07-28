use lkjscript_core::{Error, Result};

use super::{HirMemoryPlan, MemoryPlanId};

const DOMAIN: &[u8] = b"lkjscript.hir-memory-plan\0canonical-dense-records";

pub(super) fn compute_plan_id(plan: &HirMemoryPlan) -> Result<MemoryPlanId> {
    let mut bytes = Vec::new();
    frame(&mut bytes, DOMAIN)?;
    frame(&mut bytes, plan.schema.as_bytes())?;
    records(&mut bytes, &plan.functions)?;
    records(&mut bytes, &plan.entries)?;
    records(&mut bytes, &plan.uses)?;
    records(&mut bytes, &plan.loans)?;
    records(&mut bytes, &plan.constants)?;
    records(&mut bytes, &plan.calls)?;
    records(&mut bytes, &plan.obligations)?;
    records(&mut bytes, &plan.drop_glues)?;
    frame(&mut bytes, &plan.work.functions.to_be_bytes())?;
    frame(&mut bytes, &plan.work.entries.to_be_bytes())?;
    frame(&mut bytes, &plan.work.expressions.to_be_bytes())?;
    frame(&mut bytes, &plan.work.uses.to_be_bytes())?;
    frame(&mut bytes, &plan.work.loans.to_be_bytes())?;
    frame(&mut bytes, &plan.work.constants.to_be_bytes())?;
    frame(&mut bytes, &plan.work.calls.to_be_bytes())?;
    frame(&mut bytes, &plan.work.obligations.to_be_bytes())?;
    Ok(MemoryPlanId::from_bytes(lkjscript_core::sha256(&bytes)))
}

fn records<T: std::fmt::Debug>(output: &mut Vec<u8>, values: &[T]) -> Result<()> {
    frame(
        output,
        &u64::try_from(values.len())
            .map_err(|_| Error::msg("HIR memory-plan record count exceeds u64"))?
            .to_be_bytes(),
    )?;
    for value in values {
        let encoded = format!("{value:?}");
        frame(output, encoded.as_bytes())?;
    }
    Ok(())
}

fn frame(output: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    let length = u64::try_from(value.len())
        .map_err(|_| Error::msg("HIR memory-plan canonical field exceeds u64"))?;
    output
        .len()
        .checked_add(8)
        .and_then(|size| size.checked_add(value.len()))
        .ok_or_else(|| Error::msg("HIR memory-plan canonical encoding size overflow"))?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}
